//! RustForge 机器学习（IF-270 ~ IF-273，实验性 P2）：
//! 纯 Rust 张量 + MLP + SGD。桌面训练 / 各端推理。

use rf_core::Result;

/// 张量（行主序 dense）。
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

impl Tensor {
    pub fn zeros(shape: &[usize]) -> Self {
        let n: usize = shape.iter().product();
        Self { shape: shape.to_vec(), data: vec![0.0; n] }
    }

    pub fn ones(shape: &[usize]) -> Self {
        let n: usize = shape.iter().product();
        Self { shape: shape.to_vec(), data: vec![1.0; n] }
    }

    pub fn from_vec(shape: &[usize], data: Vec<f32>) -> Self {
        debug_assert_eq!(shape.iter().product::<usize>(), data.len());
        Self { shape: shape.to_vec(), data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 2D 行数。
    pub fn rows(&self) -> usize {
        self.shape.first().copied().unwrap_or(0)
    }

    /// 2D 列数。
    pub fn cols(&self) -> usize {
        self.shape.get(1).copied().unwrap_or(1)
    }

    /// 矩阵乘（[m,k]×[k,n]）。
    pub fn matmul(&self, other: &Tensor) -> Result<Tensor> {
        if self.shape.len() != 2 || other.shape.len() != 2 {
            return Err(rf_core::EngineError::InvalidData("matmul: 需要 2D".into()));
        }
        let (m, k) = (self.shape[0], self.shape[1]);
        let (k2, n) = (other.shape[0], other.shape[1]);
        if k != k2 {
            return Err(rf_core::EngineError::InvalidData(format!(
                "matmul: 形状不匹配 {m}x{k} @ {k2}x{n}"
            )));
        }
        let mut out = vec![0.0f32; m * n];
        for i in 0..m {
            for p in 0..k {
                let a = self.data[i * k + p];
                if a == 0.0 {
                    continue;
                }
                for j in 0..n {
                    out[i * n + j] += a * other.data[p * n + j];
                }
            }
        }
        Ok(Tensor { shape: vec![m, n], data: out })
    }

    pub fn transposed(&self) -> Tensor {
        if self.shape.len() != 2 {
            return self.clone();
        }
        let (m, n) = (self.shape[0], self.shape[1]);
        let mut out = vec![0.0; m * n];
        for i in 0..m {
            for j in 0..n {
                out[j * m + i] = self.data[i * n + j];
            }
        }
        Tensor { shape: vec![n, m], data: out }
    }

    /// 逐元素加。
    pub fn element_add(&self, other: &Tensor) -> Result<Tensor> {
        if self.shape != other.shape {
            return Err(rf_core::EngineError::InvalidData("element_add: 形状不一致".into()));
        }
        Ok(Tensor {
            shape: self.shape.clone(),
            data: self.data.iter().zip(&other.data).map(|(a, b)| a + b).collect(),
        })
    }

    /// 标量乘。
    pub fn scale(&self, s: f32) -> Tensor {
        Tensor { shape: self.shape.clone(), data: self.data.iter().map(|v| v * s).collect() }
    }

    /// argmax（按最后维展开）。
    pub fn argmax(&self) -> usize {
        let mut best = 0usize;
        let mut best_v = f32::NEG_INFINITY;
        for (i, v) in self.data.iter().enumerate() {
            if *v > best_v {
                best_v = *v;
                best = i;
            }
        }
        best
    }

    /// MSE。
    pub fn mse(&self, other: &Tensor) -> f32 {
        let n = self.data.len().max(1);
        self.data.iter().zip(&other.data).map(|(a, b)| (a - b) * (a - b)).sum::<f32>() / n as f32
    }

    pub fn rmse(&self, other: &Tensor) -> f32 {
        self.mse(other).sqrt()
    }
}

/// 激活函数（IF-272）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Activation {
    Identity,
    ReLU,
    Sigmoid,
    Tanh,
}

impl Activation {
    pub fn fn_(&self, x: f32) -> f32 {
        match self {
            Activation::Identity => x,
            Activation::ReLU => x.max(0.0),
            Activation::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            Activation::Tanh => x.tanh(),
        }
    }

    pub fn derivative(&self, x: f32) -> f32 {
        match self {
            Activation::Identity => 1.0,
            Activation::ReLU => {
                if x > 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            Activation::Sigmoid => {
                let s = self.fn_(x);
                s * (1.0 - s)
            }
            Activation::Tanh => {
                let t = x.tanh();
                1.0 - t * t
            }
        }
    }
}

/// 全连接层。
pub struct Dense {
    pub w: Tensor,
    pub b: Tensor,
    pub activation: Activation,
}

impl Dense {
    /// He/Xavier 初始化（确定性种子）。
    pub fn new(in_dim: usize, out_dim: usize, activation: Activation, seed: u64) -> Self {
        let mut rng = rf_core::Pcg32::new(seed);
        let scale = (2.0 / in_dim as f32).sqrt();
        let w: Vec<f32> =
            (0..in_dim * out_dim).map(|_| (rng.next_f32() * 2.0 - 1.0) * scale).collect();
        Self {
            w: Tensor::from_vec(&[in_dim, out_dim], w),
            b: Tensor::zeros(&[1, out_dim]),
            activation,
        }
    }

    pub fn forward(&self, x: &Tensor) -> Result<(Tensor, Tensor)> {
        let mut z = x.matmul(&self.w)?;
        z = z.element_add(&self.b.broadcast_rows(z.rows()))?;
        let a = Tensor {
            shape: z.shape.clone(),
            data: z.data.iter().map(|v| self.activation.fn_(*v)).collect(),
        };
        Ok((z, a))
    }
}

impl Tensor {
    /// [1,n] → [rows,n] 广播。
    fn broadcast_rows(&self, rows: usize) -> Tensor {
        if self.rows() == rows {
            return self.clone();
        }
        let n = self.cols();
        let mut data = Vec::with_capacity(rows * n);
        for _ in 0..rows {
            data.extend_from_slice(&self.data);
        }
        Tensor { shape: vec![rows, n], data }
    }
}

/// 训练报告（IF-271）。
#[derive(Debug, Clone)]
pub struct TrainingReport {
    pub epochs: u32,
    pub final_loss: f32,
    pub history: Vec<f32>,
}

/// ML 后端 trait（IF-271）。
pub trait MlBackend {
    fn name(&self) -> &str;
    fn train(&mut self, samples: &[(Tensor, Tensor)], epochs: u32, lr: f32) -> TrainingReport;
    fn infer(&mut self, input: &Tensor) -> Result<Tensor>;
    fn save(&self, path: &str) -> Result<()>;
    fn load(&mut self, path: &str) -> Result<()>;
}

/// 纯 Rust MLP（SGD + MSE，IF-273）。
pub struct MlpBackend {
    layers: Vec<Dense>,
}

impl MlpBackend {
    /// dims: [in, hidden..., out]；activations 含输出层。
    pub fn new(dims: &[usize], activations: &[Activation], seed: u64) -> Self {
        let mut layers = Vec::with_capacity(dims.len() - 1);
        for i in 0..dims.len() - 1 {
            layers.push(Dense::new(dims[i], dims[i + 1], activations[i], seed + i as u64));
        }
        Self { layers }
    }

    fn forward_all(&self, x: &Tensor) -> Result<(Vec<Tensor>, Vec<Tensor>)> {
        let mut zs = Vec::new();
        let mut as_ = vec![x.clone()];
        let mut cur = x.clone();
        for layer in &self.layers {
            let (z, a) = layer.forward(&cur)?;
            zs.push(z);
            as_.push(a.clone());
            cur = a;
        }
        Ok((zs, as_))
    }

    /// 反向传播（单样本；SGD 逐样本更新）。
    fn train_step(&mut self, x: &Tensor, y: &Tensor, lr: f32) -> f32 {
        let (zs, as_) = match self.forward_all(x) {
            Ok(v) => v,
            Err(_) => return 0.0,
        };
        let loss = as_.last().unwrap().mse(y);
        // 输出层 delta = (a - y) * act'(z)
        let out = as_.last().unwrap();
        let z_last = zs.last().unwrap();
        let act = self.layers.last().unwrap().activation;
        let mut delta: Vec<f32> = out
            .data
            .iter()
            .zip(&y.data)
            .zip(&z_last.data)
            .map(|((a, t), z)| (a - t) * act.derivative(*z))
            .collect();
        // 反向逐层
        for li in (0..self.layers.len()).rev() {
            let a_prev = &as_[li];
            let (m, n) = (a_prev.cols(), self.layers[li].w.cols());
            let _ = m;
            // 梯度：dW = a_prev^T · delta；db = delta
            let mut dw =
                vec![0.0f32; a_prev.len() * delta.len() / delta.len() * self.layers[li].w.len()];
            dw.resize(self.layers[li].w.len(), 0.0);
            for (oi, &d) in delta.iter().enumerate() {
                for (ii, &ap) in a_prev.data.iter().enumerate() {
                    dw[ii * delta.len() + oi] += ap * d;
                }
            }
            let db = delta.clone();
            // 更新
            for (wv, g) in self.layers[li].w.data.iter_mut().zip(&dw) {
                *wv -= lr * g;
            }
            for (bv, g) in self.layers[li].b.data.iter_mut().zip(&db) {
                *bv -= lr * g;
            }
            let _ = n;
            // 传到前一层
            if li > 0 {
                let w = &self.layers[li].w;
                let prev_dim = w.rows();
                let mut next_delta = vec![0.0; prev_dim];
                for (pi, nd) in next_delta.iter_mut().enumerate() {
                    let mut s = 0.0;
                    for (oi, &d) in delta.iter().enumerate() {
                        s += d * w.data[pi * delta.len() + oi];
                    }
                    *nd = s;
                }
                let z_prev = &zs[li - 1];
                let act_prev = self.layers[li - 1].activation;
                for (nd, z) in next_delta.iter_mut().zip(&z_prev.data) {
                    *nd *= act_prev.derivative(*z);
                }
                delta = next_delta;
            }
        }
        loss
    }
}

impl MlBackend for MlpBackend {
    fn name(&self) -> &str {
        "mlp-pure-rust"
    }

    fn train(&mut self, samples: &[(Tensor, Tensor)], epochs: u32, lr: f32) -> TrainingReport {
        let mut history = Vec::new();
        let mut rng = rf_core::Pcg32::new(0xFEED);
        for _ in 0..epochs {
            let mut total = 0.0f32;
            // 随机打乱（确定性）
            let mut order: Vec<usize> = (0..samples.len()).collect();
            for i in (1..order.len()).rev() {
                let j = rng.range(0, i as i32) as usize;
                order.swap(i, j);
            }
            for &i in &order {
                total += self.train_step(&samples[i].0, &samples[i].1, lr);
            }
            history.push(total / samples.len().max(1) as f32);
        }
        TrainingReport { epochs, final_loss: history.last().copied().unwrap_or(0.0), history }
    }

    fn infer(&mut self, input: &Tensor) -> Result<Tensor> {
        let (_, as_) = self.forward_all(input)?;
        Ok(as_.last().unwrap().clone())
    }

    fn save(&self, path: &str) -> Result<()> {
        let mut out = String::new();
        for l in &self.layers {
            out.push_str(&format!("layer {} {} {:?}\n", l.w.rows(), l.w.cols(), l.activation));
            out.push_str(&l.w.data.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(","));
            out.push('\n');
            out.push_str(&l.b.data.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(","));
            out.push('\n');
        }
        std::fs::write(path, out).map_err(rf_core::EngineError::Io)
    }

    fn load(&mut self, path: &str) -> Result<()> {
        let text = std::fs::read_to_string(path).map_err(rf_core::EngineError::Io)?;
        let mut lines = text.lines();
        let mut layers = Vec::new();
        while let Some(header) = lines.next() {
            let parts: Vec<&str> = header.split_whitespace().collect();
            if parts.len() < 4 || parts[0] != "layer" {
                continue;
            }
            let rows: usize = parts[1]
                .parse()
                .map_err(|_| rf_core::EngineError::InvalidData("bad rows".into()))?;
            let cols: usize = parts[2]
                .parse()
                .map_err(|_| rf_core::EngineError::InvalidData("bad cols".into()))?;
            let act = match parts[3] {
                "ReLU" => Activation::ReLU,
                "Sigmoid" => Activation::Sigmoid,
                "Tanh" => Activation::Tanh,
                _ => Activation::Identity,
            };
            let w_line = lines
                .next()
                .ok_or_else(|| rf_core::EngineError::InvalidData("missing w".into()))?;
            let b_line = lines
                .next()
                .ok_or_else(|| rf_core::EngineError::InvalidData("missing b".into()))?;
            let w: Vec<f32> = w_line.split(',').filter_map(|v| v.parse().ok()).collect();
            let b: Vec<f32> = b_line.split(',').filter_map(|v| v.parse().ok()).collect();
            if w.len() != rows * cols {
                return Err(rf_core::EngineError::InvalidData("w size mismatch".into()));
            }
            layers.push(Dense {
                w: Tensor::from_vec(&[rows, cols], w),
                b: Tensor::from_vec(&[1, b.len()], b),
                activation: act,
            });
        }
        if layers.is_empty() {
            return Err(rf_core::EngineError::InvalidData("empty model".into()));
        }
        self.layers = layers;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(in_dim: usize, out_dim: usize, x: Vec<f32>, y: Vec<f32>) -> (Tensor, Tensor) {
        (Tensor::from_vec(&[1, in_dim], x), Tensor::from_vec(&[1, out_dim], y))
    }

    #[test]
    fn tensor_ops() {
        let a = Tensor::from_vec(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let b = Tensor::from_vec(&[3, 2], vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]);
        let c = a.matmul(&b).unwrap();
        assert_eq!(c.shape, vec![2, 2]);
        assert_eq!(c.data, vec![58.0, 64.0, 139.0, 154.0]);
        let t = a.transposed();
        assert_eq!(t.shape, vec![3, 2]);
        assert_eq!(t.data[1], 4.0); // t = [[1,4],[2,5],[3,6]]
        assert_eq!(t.data[2], 2.0);
        assert!(a.matmul(&a).is_err()); // 形状不匹配（2x3 @ 2x3）
        assert_eq!(a.element_add(&a).unwrap().data[0], 2.0);
        assert_eq!(a.scale(2.0).data[1], 4.0);
        assert_eq!(Tensor::from_vec(&[1, 3], vec![0.1, 0.9, 0.3]).argmax(), 1);
        let m = a.mse(&a);
        assert_eq!(m, 0.0);
    }

    #[test]
    fn activation_derivatives() {
        assert_eq!(Activation::ReLU.fn_(-1.0), 0.0);
        assert_eq!(Activation::ReLU.derivative(2.0), 1.0);
        assert!((Activation::Sigmoid.fn_(0.0) - 0.5).abs() < 1e-6);
        assert!((Activation::Tanh.fn_(0.0)).abs() < 1e-6);
        let s = Activation::Sigmoid.fn_(1.0);
        assert!((Activation::Sigmoid.derivative(1.0) - s * (1.0 - s)).abs() < 1e-6);
    }

    #[test]
    fn xor_training_converges() {
        // 实验性验收：MLP 学会 XOR（规格 I2：训练为实验特性）
        let samples = vec![
            sample(2, 1, vec![0.0, 0.0], vec![0.0]),
            sample(2, 1, vec![0.0, 1.0], vec![1.0]),
            sample(2, 1, vec![1.0, 0.0], vec![1.0]),
            sample(2, 1, vec![1.0, 1.0], vec![0.0]),
        ];
        let mut mlp = MlpBackend::new(&[2, 4, 1], &[Activation::Tanh, Activation::Sigmoid], 42);
        let report = mlp.train(&samples, 3000, 0.5);
        assert!(report.final_loss < 0.05, "loss {}", report.final_loss);
        assert_eq!(report.history.len(), 3000);
        for (x, y) in &samples {
            let out = mlp.infer(x).unwrap();
            assert!((out.data[0] - y.data[0]).abs() < 0.25, "out {} vs {}", out.data[0], y.data[0]);
        }
    }

    #[test]
    fn save_load_roundtrip() {
        let mut mlp = MlpBackend::new(&[2, 3, 1], &[Activation::ReLU, Activation::Identity], 7);
        let x = Tensor::from_vec(&[1, 2], vec![1.0, 2.0]);
        let before = mlp.infer(&x).unwrap();
        let dir = std::env::temp_dir().join("rf_ml_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.txt");
        mlp.save(path.to_str().unwrap()).unwrap();
        let mut mlp2 = MlpBackend::new(&[2, 8, 1], &[Activation::Tanh, Activation::Sigmoid], 99);
        mlp2.load(path.to_str().unwrap()).unwrap();
        let after = mlp2.infer(&x).unwrap();
        assert_eq!(before.data, after.data); // 权重一致 → 输出一致
        assert!(mlp2.load("nonexistent-file.model").is_err());
    }

    #[test]
    fn deterministic_training() {
        let samples = vec![sample(1, 1, vec![0.5], vec![1.0]), sample(1, 1, vec![1.5], vec![0.0])];
        let run = || {
            let mut m = MlpBackend::new(&[1, 3, 1], &[Activation::Tanh, Activation::Sigmoid], 11);
            m.train(&samples, 100, 0.3).final_loss
        };
        assert_eq!(run(), run()); // 同种子同结果
    }
}
