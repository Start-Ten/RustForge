//! RustForge 音频（IF-220 ~ IF-223）：DSP 节点图、混音、空间化、WAV 编解码。
//! P0 交付离线渲染（render_to_wav）；实时设备输出列 P1（cpal）。

use rf_core::Result;
use std::sync::Arc;

/// 音频图（IF-220）。
pub trait AudioGraph {
    /// 渲染一段样本（交错立体声）。
    fn render(&mut self, out: &mut [f32], rate: u32);
}

/// 音频资产（IF-162：rf-asset 导入产出）。
pub struct AudioAsset {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
}

/// 振荡器类型。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OscKind {
    Sine,
    Square,
    Saw,
    Triangle,
    Noise,
}

/// 节点参数（值型简化：MVP 为链式拓扑，P1 升级任意图连接）。
#[derive(Debug, Clone)]
pub enum AudioNode {
    Oscillator { kind: OscKind, freq: f32, gain: f32, phase: f32 },
    Sampler { samples: Arc<Vec<f32>>, pos: usize, rate_ratio: f32, looping: bool, gain: f32 },
    Gain(f32),
    Pan(f32),
    Lowpass { cutoff: f32, state: f32 },
    Highpass { cutoff: f32, prev_in: f32, prev_out: f32 },
    Envelope { attack: f32, decay: f32, sustain: f32, release: f32, time: f32, gate: bool },
    Delay { mix: f32, buffer: Vec<f32>, pos: usize },
}

impl AudioNode {
    pub fn oscillator(kind: OscKind, freq: f32, gain: f32) -> Self {
        AudioNode::Oscillator { kind, freq, gain, phase: 0.0 }
    }

    pub fn sampler(samples: Arc<Vec<f32>>, rate_ratio: f32, looping: bool, gain: f32) -> Self {
        AudioNode::Sampler { samples, pos: 0, rate_ratio, looping, gain }
    }

    /// 处理单样本（mono），内部推进状态。
    fn process(&mut self, input: f32, rate: f32, rng: &mut u32) -> f32 {
        match self {
            AudioNode::Oscillator { kind, freq, gain, phase } => {
                *phase += *freq / rate;
                if *phase >= 1.0 {
                    *phase -= 1.0;
                }
                let v = match kind {
                    OscKind::Sine => (*phase * std::f32::consts::TAU).sin(),
                    OscKind::Square => {
                        if *phase < 0.5 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    OscKind::Saw => 2.0 * *phase - 1.0,
                    OscKind::Triangle => 4.0 * (*phase - 0.5).abs() - 1.0,
                    OscKind::Noise => {
                        // xorshift 噪声
                        *rng ^= *rng << 13;
                        *rng ^= *rng >> 17;
                        *rng ^= *rng << 5;
                        *rng as f32 / u32::MAX as f32 * 2.0 - 1.0
                    }
                };
                v * *gain
            }
            AudioNode::Sampler { samples, pos, rate_ratio, looping, gain } => {
                let idx = *pos as f32;
                let i0 = idx as usize;
                if i0 + 1 >= samples.len() {
                    if *looping && !samples.is_empty() {
                        *pos = 0;
                        return samples[0] * *gain;
                    }
                    return 0.0;
                }
                let frac = idx - i0 as f32;
                let v = samples[i0] * (1.0 - frac) + samples[i0 + 1] * frac;
                *pos = (idx + *rate_ratio) as usize;
                v * *gain
            }
            AudioNode::Gain(g) => input * *g,
            AudioNode::Pan(p) => input * (1.0 - p.abs()), // mono 链内衰减；声像在混音器级
            AudioNode::Lowpass { cutoff, state } => {
                let a = (std::f32::consts::TAU * *cutoff / rate).tanh(); // 一阶 IIR
                *state += a * (input - *state);
                *state
            }
            AudioNode::Highpass { cutoff, prev_in, prev_out } => {
                let a = (std::f32::consts::TAU * *cutoff / rate).tanh();
                let out = a * (*prev_out + input - *prev_in);
                *prev_in = input;
                *prev_out = out;
                out
            }
            AudioNode::Envelope { attack, decay, sustain, release, time, gate } => {
                *time += 1.0 / rate;
                if *gate {
                    if *time < *attack {
                        *time / attack.max(f32::EPSILON)
                    } else if *time < *attack + *decay {
                        let d = (*time - *attack) / decay.max(f32::EPSILON);
                        1.0 - d * (1.0 - *sustain)
                    } else {
                        *sustain
                    }
                } else {
                    (*sustain * (1.0 - *time / release.max(f32::EPSILON))).max(0.0)
                }
            }
            AudioNode::Delay { mix, buffer, pos } => {
                let delayed = buffer[*pos];
                let out = input + delayed * *mix;
                buffer[*pos] = out;
                *pos = (*pos + 1) % buffer.len().max(1);
                out
            }
        }
    }
}

/// 混音图（IF-222）：节点链（源 → 效果串）→ 总线，立体声输出。
pub struct MixerGraph {
    rate: u32,
    chains: Vec<Vec<AudioNode>>,
    gains: Vec<f32>,
    pans: Vec<f32>,
    rng: u32,
}

impl MixerGraph {
    pub fn new(rate: u32) -> Self {
        Self { rate, chains: Vec::new(), gains: Vec::new(), pans: Vec::new(), rng: 0x9E3779B9 }
    }

    /// 添加节点链，返回链 id。
    pub fn add_chain(&mut self, nodes: Vec<AudioNode>) -> usize {
        self.chains.push(nodes);
        self.gains.push(1.0);
        self.pans.push(0.0);
        self.chains.len() - 1
    }

    /// 连接效果（链尾追加）。
    pub fn connect(&mut self, chain: usize, node: AudioNode) {
        if let Some(c) = self.chains.get_mut(chain) {
            c.push(node);
        }
    }

    /// 设置参数：节点索引.参数名。
    pub fn set_param(&mut self, chain: usize, node: usize, param: &str, value: f32) {
        let Some(c) = self.chains.get_mut(chain) else { return };
        let Some(n) = c.get_mut(node) else { return };
        match n {
            AudioNode::Oscillator { freq, gain, .. } => match param {
                "freq" => *freq = value,
                "gain" => *gain = value,
                _ => {}
            },
            AudioNode::Sampler { gain, .. } => {
                if param == "gain" {
                    *gain = value;
                }
            }
            AudioNode::Gain(g) => *g = value,
            AudioNode::Pan(p) => *p = value,
            AudioNode::Lowpass { cutoff, .. } => *cutoff = value,
            AudioNode::Highpass { cutoff, .. } => *cutoff = value,
            AudioNode::Delay { mix, .. } => *mix = value,
            _ => {}
        }
    }
}

impl AudioGraph for MixerGraph {
    fn render(&mut self, out: &mut [f32], rate: u32) {
        let _ = rate; // 使用构造时的 rate
        let gains = self.gains.clone();
        let pans = self.pans.clone();
        for frame in out.chunks_exact_mut(2) {
            let mut left = 0.0f32;
            let mut right = 0.0f32;
            for (i, chain) in self.chains.iter_mut().enumerate() {
                let mut v = 0.0f32;
                for node in chain.iter_mut() {
                    v = node.process(v, self.rate as f32, &mut self.rng);
                }
                v *= gains.get(i).copied().unwrap_or(1.0);
                let pan = pans.get(i).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
                left += v * (1.0 - pan.max(0.0));
                right += v * (1.0 + pan.min(0.0));
            }
            frame[0] = left.clamp(-1.0, 1.0);
            frame[1] = right.clamp(-1.0, 1.0);
        }
    }
}

/// 渲染为 WAV 字节（16-bit PCM 交织）。
pub fn render_to_wav(graph: &mut MixerGraph, seconds: f32, rate: u32) -> Vec<u8> {
    let frames = (seconds * rate as f32) as usize;
    let mut buf = vec![0f32; frames * 2];
    graph.render(&mut buf, rate);
    encode_wav(&buf, rate)
}

/// f32 交织样本 → 16-bit PCM WAV。
pub fn encode_wav(interleaved: &[f32], rate: u32) -> Vec<u8> {
    let channels: u16 = 2;
    let bits: u16 = 16;
    let data_len = interleaved.len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * channels as u32 * bits as u32 / 8).to_le_bytes());
    out.extend_from_slice(&(channels * bits / 8).to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in interleaved {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// 16-bit PCM WAV → f32 交织样本。
pub fn decode_wav(bytes: &[u8]) -> Result<(Vec<f32>, u32)> {
    let err = |m: &str| rf_core::EngineError::InvalidData(format!("wav: {m}"));
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(err("bad header"));
    }
    let mut pos = 12;
    let mut rate = 44_100u32;
    let mut channels = 2u16;
    let mut bits = 16u16;
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body_start = pos + 8;
        let body_end = (body_start + size).min(bytes.len());
        match id {
            b"fmt " => {
                if body_end - body_start >= 16 {
                    channels = u16::from_le_bytes(
                        bytes[body_start + 2..body_start + 4].try_into().unwrap(),
                    );
                    rate = u32::from_le_bytes(
                        bytes[body_start + 4..body_start + 8].try_into().unwrap(),
                    );
                    bits = u16::from_le_bytes(
                        bytes[body_start + 14..body_start + 16].try_into().unwrap(),
                    );
                }
            }
            b"data" => data = Some(&bytes[body_start..body_end]),
            _ => {}
        }
        pos = body_start + size + (size & 1);
    }
    let data = data.ok_or_else(|| err("missing data chunk"))?;
    let out: Vec<f32> = match bits {
        16 => data
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32767.0)
            .collect(),
        8 => data.iter().map(|b| (*b as f32 - 128.0) / 128.0).collect(),
        _ => return Err(err("unsupported bit depth")),
    };
    let _ = channels;
    Ok((out, rate))
}

/// 空间化（IF-223）：距离衰减 + 左右声像。
pub fn spatialize(
    listener: rf_math::Vec2,
    facing: f32,
    source: rf_math::Vec2,
    max_dist: f32,
) -> (f32, f32) {
    let delta = source - listener;
    let dist = delta.length();
    let atten = 1.0 - (dist / max_dist.max(f32::EPSILON)).clamp(0.0, 1.0);
    // 声像：源方向相对朝向的角度
    let angle = delta.angle() - facing;
    let pan = (angle.sin() * 0.5).clamp(-0.5, 0.5);
    let l = atten * (1.0 - pan.max(0.0));
    let r = atten * (1.0 + pan.min(0.0));
    (l.clamp(0.0, 1.0), r.clamp(0.0, 1.0))
}

/// 混响近似（简单 comb：多次延迟叠加）。
pub fn simple_reverb(samples: &mut [f32], delay_samples: usize, feedback: f32) {
    if delay_samples == 0 || samples.len() <= delay_samples {
        return;
    }
    for i in delay_samples..samples.len() {
        samples[i] += samples[i - delay_samples] * feedback;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oscillator_shapes() {
        let mut g = MixerGraph::new(48_000);
        g.add_chain(vec![AudioNode::oscillator(OscKind::Sine, 440.0, 0.5)]);
        let mut buf = [0f32; 8];
        g.render(&mut buf, 48_000);
        assert!(buf.iter().any(|v| *v != 0.0));
        // 方波链
        let mut g2 = MixerGraph::new(48_000);
        g2.add_chain(vec![AudioNode::oscillator(OscKind::Square, 100.0, 1.0)]);
        let mut buf2 = [0f32; 4];
        g2.render(&mut buf2, 48_000);
        assert!((buf2[0] - 1.0).abs() < 1e-6 || (buf2[0] + 1.0).abs() < 1e-6);
    }

    #[test]
    fn wav_roundtrip() {
        let mut g = MixerGraph::new(8000);
        g.add_chain(vec![AudioNode::oscillator(OscKind::Sine, 440.0, 0.8)]);
        let wav = render_to_wav(&mut g, 0.1, 8000);
        assert_eq!(&wav[0..4], b"RIFF");
        let (samples, rate) = decode_wav(&wav).unwrap();
        assert_eq!(rate, 8000);
        assert_eq!(samples.len(), 800 * 2);
        assert!(samples.iter().any(|s| s.abs() > 0.3)); // 信号存在
    }

    #[test]
    fn bad_wav_rejected() {
        assert!(decode_wav(b"not a wav").is_err());
        assert!(decode_wav(&[]).is_err());
    }

    #[test]
    fn spatial_attenuation() {
        let (l, r) = spatialize(rf_math::Vec2::ZERO, 0.0, rf_math::Vec2::new(0.0, 10.0), 20.0);
        assert!(l > 0.0 && r > 0.0);
        let (l2, _) = spatialize(rf_math::Vec2::ZERO, 0.0, rf_math::Vec2::new(0.0, 10.0), 5.0);
        assert_eq!(l2, 0.0); // 超出最大距离
                             // 正前方对称
        let (l3, r3) = spatialize(rf_math::Vec2::ZERO, 0.0, rf_math::Vec2::new(5.0, 0.0), 10.0);
        assert!((l3 - r3).abs() < 1e-5);
    }

    #[test]
    fn lowpass_smoothes() {
        let mut chain = [AudioNode::Lowpass { cutoff: 200.0, state: 0.0 }];
        let mut rng = 0x1234;
        let mut prev = 0.0f32;
        let mut max_delta = 0.0f32;
        for i in 0..1000 {
            let input = if i % 2 == 0 { 1.0 } else { -1.0 };
            let out = chain[0].process(input, 44_100.0, &mut rng);
            max_delta = max_delta.max((out - prev).abs());
            prev = out;
        }
        assert!(max_delta < 0.5); // 方波被平滑
    }
}
