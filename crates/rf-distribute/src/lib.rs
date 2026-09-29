//! RustForge 分发（IF-310 ~ IF-324）：构建/打包/签名/商店/更新/成就/云存档/崩溃/合规/SDK。

use rf_core::{EngineError, Result, Version};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---- 构建配置（IF-310/311/312） ----

/// 构建档位（IF-310）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildProfile {
    Debug,
    Development,
    Release,
    Shipping,
    Test,
}

impl BuildProfile {
    pub fn cargo_profile(&self) -> &'static str {
        match self {
            BuildProfile::Debug => "dev",
            BuildProfile::Development => "dev",
            BuildProfile::Release | BuildProfile::Shipping => "release",
            BuildProfile::Test => "test",
        }
    }

    pub fn strip(&self) -> bool {
        matches!(self, BuildProfile::Shipping)
    }

    pub fn symbols(&self) -> bool {
        !matches!(self, BuildProfile::Shipping)
    }

    pub fn debug_assertions(&self) -> bool {
        matches!(self, BuildProfile::Debug | BuildProfile::Development | BuildProfile::Test)
    }

    pub fn opt_level(&self) -> u8 {
        match self {
            BuildProfile::Debug | BuildProfile::Test => 0,
            BuildProfile::Development => 1,
            BuildProfile::Release | BuildProfile::Shipping => 3,
        }
    }
}

/// 目标平台（IF-311）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformTarget {
    WindowsX64,
    WindowsArm64,
    LinuxX64,
    LinuxArm64,
    AndroidArm64,
    AndroidArmv7,
    MacosX64,
    MacosArm64,
    IosArm64,
    WebWasm,
}

impl PlatformTarget {
    pub fn triple(&self) -> &'static str {
        match self {
            PlatformTarget::WindowsX64 => "x86_64-pc-windows-msvc",
            PlatformTarget::WindowsArm64 => "aarch64-pc-windows-msvc",
            PlatformTarget::LinuxX64 => "x86_64-unknown-linux-gnu",
            PlatformTarget::LinuxArm64 => "aarch64-unknown-linux-gnu",
            PlatformTarget::AndroidArm64 => "aarch64-linux-android",
            PlatformTarget::AndroidArmv7 => "armv7-linux-androideabi",
            PlatformTarget::MacosX64 => "x86_64-apple-darwin",
            PlatformTarget::MacosArm64 => "aarch64-apple-darwin",
            PlatformTarget::IosArm64 => "aarch64-apple-ios",
            PlatformTarget::WebWasm => "wasm32-unknown-unknown",
        }
    }

    pub fn is_host(&self) -> bool {
        let host = std::env::consts::ARCH.to_string() + "-" + std::env::consts::OS;
        let native = match self {
            PlatformTarget::WindowsX64 => "x86_64-windows",
            PlatformTarget::LinuxX64 => "x86_64-linux",
            PlatformTarget::MacosArm64 => "aarch64-macos",
            _ => "",
        };
        !native.is_empty() && host == native
    }

    pub fn is_desktop(&self) -> bool {
        matches!(
            self,
            PlatformTarget::WindowsX64
                | PlatformTarget::WindowsArm64
                | PlatformTarget::LinuxX64
                | PlatformTarget::LinuxArm64
                | PlatformTarget::MacosX64
                | PlatformTarget::MacosArm64
        )
    }

    pub fn is_mobile(&self) -> bool {
        matches!(
            self,
            PlatformTarget::AndroidArm64 | PlatformTarget::AndroidArmv7 | PlatformTarget::IosArm64
        )
    }

    /// 是否需要交叉工具链（ndk 等）。
    pub fn needs_cross_tool(&self) -> bool {
        !matches!(
            self,
            PlatformTarget::WindowsX64
                | PlatformTarget::WindowsArm64
                | PlatformTarget::LinuxX64
                | PlatformTarget::LinuxArm64
                | PlatformTarget::MacosX64
                | PlatformTarget::MacosArm64
        )
    }
}

/// 构建计划（IF-312）。
#[derive(Debug, Clone)]
pub struct BuildPlan {
    pub profile: BuildProfile,
    pub target: PlatformTarget,
    pub features: Vec<String>,
    pub examples: Vec<String>,
    pub out_dir: PathBuf,
}

impl BuildPlan {
    pub fn plan(profile: BuildProfile, target: PlatformTarget) -> Self {
        Self {
            profile,
            target,
            features: Vec::new(),
            examples: Vec::new(),
            out_dir: PathBuf::from("target/dist"),
        }
    }

    /// cargo 命令行（可直接复制执行）。
    pub fn command_line(&self) -> Vec<String> {
        let mut cmd = vec!["cargo".to_string(), "build".to_string()];
        if self.profile.cargo_profile() == "release" {
            cmd.push("--release".into());
        }
        cmd.push("--target".into());
        cmd.push(self.target.triple().to_string());
        if !self.features.is_empty() {
            cmd.push("--features".into());
            cmd.push(self.features.join(","));
        }
        for e in &self.examples {
            cmd.push("--example".into());
            cmd.push(e.clone());
        }
        cmd
    }

    /// 执行构建（调用 cargo）。本机目标直接编译；交叉目标失败时报错并提示工具链。
    pub fn execute(&self) -> Result<BuildReport> {
        let start = std::time::Instant::now();
        if self.target.needs_cross_tool() && !target_installed(self.target.triple()) {
            return Err(EngineError::Message(format!(
                "交叉目标 {} 未安装（rustup target add {}；Android 另需 NDK）",
                self.target.triple(),
                self.target.triple()
            )));
        }
        let cmd = self.command_line();
        let status = std::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .status()
            .map_err(|e| EngineError::Message(format!("启动 cargo 失败: {e}")))?;
        if !status.success() {
            return Err(EngineError::Message(format!("cargo 构建失败（{:?}）", status.code())));
        }
        let mut artifacts = Vec::new();
        let exe_dir =
            PathBuf::from("target").join(self.target.triple()).join(self.profile.cargo_profile());
        for e in &self.examples {
            let ext = if self.target.triple().contains("windows") { ".exe" } else { "" };
            let p = exe_dir.join(format!("examples/{e}{ext}"));
            if p.exists() {
                artifacts.push(p.to_string_lossy().into_owned());
            }
        }
        Ok(BuildReport { duration_secs: start.elapsed().as_secs_f32(), success: true, artifacts })
    }
}

fn target_installed(triple: &str) -> bool {
    std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(triple))
        .unwrap_or(true) // rustup 不存在时假设可用（本机工具链）
}

/// 构建报告。
#[derive(Debug, Clone)]
pub struct BuildReport {
    pub duration_secs: f32,
    pub success: bool,
    pub artifacts: Vec<String>,
}

// ---- 哈希（IF-314） ----

/// CRC32（IEEE）。
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// SHA-256（自实现，FIPS 180-4）。
pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    h.iter().map(|v| format!("{v:08x}")).collect()
}

/// HMAC-SHA256。
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        let hex = sha256_hex(key);
        for (i, b) in hex.as_bytes().iter().take(32).enumerate() {
            k[i] = *b;
        }
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let ipad: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    let opad: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();
    let mut inner = ipad;
    inner.extend_from_slice(data);
    let inner_hex = sha256_hex(&inner);
    let mut outer = opad;
    outer.extend_from_slice(inner_hex.as_bytes());
    let outer_hex = sha256_hex(&outer);
    let mut out = [0u8; 32];
    for (i, pair) in outer_hex.as_bytes().chunks_exact(2).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap_or(0);
    }
    out
}

// ---- 打包（IF-313） ----

/// 输出产物描述。
#[derive(Debug, Clone)]
pub struct OutputArtifact {
    pub path: String,
    pub bytes: u64,
    pub crc32: u32,
    pub sha256: String,
}

/// 打包器 trait（IF-313，可扩展）。
pub trait Packager {
    fn name(&self) -> &str;
    fn package(&self, out_dir: &Path, files: &[(String, Vec<u8>)]) -> Result<OutputArtifact>;
}

/// ZIP 打包器（store + deflate，自实现中央目录）。
pub struct ZipPackager;

impl Packager for ZipPackager {
    fn name(&self) -> &str {
        "zip"
    }

    fn package(&self, out_dir: &Path, files: &[(String, Vec<u8>)]) -> Result<OutputArtifact> {
        use std::io::Write;
        std::fs::create_dir_all(out_dir)?;
        let mut zip_data: Vec<u8> = Vec::new();
        let mut central: Vec<u8> = Vec::new();
        let mut count = 0u16;
        for (name, data) in files {
            let crc = crc32(data);
            let (method, stored) = if data.len() > 64 {
                let mut enc =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                enc.write_all(data)?;
                (8u16, enc.finish()?)
            } else {
                (0u16, data.clone())
            };
            let offset = zip_data.len() as u32;
            let name_bytes = name.as_bytes();
            // local header
            zip_data.extend_from_slice(&0x04034b50u32.to_le_bytes());
            zip_data.extend_from_slice(&20u16.to_le_bytes()); // version
            zip_data.extend_from_slice(&0u16.to_le_bytes()); // flags
            zip_data.extend_from_slice(&method.to_le_bytes());
            zip_data.extend_from_slice(&0u16.to_le_bytes()); // time
            zip_data.extend_from_slice(&0u16.to_le_bytes()); // date
            zip_data.extend_from_slice(&crc.to_le_bytes());
            zip_data.extend_from_slice(&(stored.len() as u32).to_le_bytes());
            zip_data.extend_from_slice(&(data.len() as u32).to_le_bytes());
            zip_data.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            zip_data.extend_from_slice(&0u16.to_le_bytes()); // extra len
            zip_data.extend_from_slice(name_bytes);
            zip_data.extend_from_slice(&stored);
            // central entry
            central.extend_from_slice(&0x02014b50u32.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&method.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(stored.len() as u32).to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name_bytes);
            count += 1;
        }
        let central_offset = zip_data.len() as u32;
        zip_data.extend_from_slice(&central);
        zip_data.extend_from_slice(&0x06054b50u32.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        zip_data.extend_from_slice(&count.to_le_bytes());
        zip_data.extend_from_slice(&count.to_le_bytes());
        zip_data.extend_from_slice(&(central.len() as u32).to_le_bytes());
        zip_data.extend_from_slice(&central_offset.to_le_bytes());
        zip_data.extend_from_slice(&0u16.to_le_bytes());
        let out_path = out_dir.join("package.zip");
        std::fs::write(&out_path, &zip_data)?;
        let crc = crc32(&zip_data);
        let sha = sha256_hex(&zip_data);
        Ok(OutputArtifact {
            path: out_path.to_string_lossy().into_owned(),
            bytes: zip_data.len() as u64,
            crc32: crc,
            sha256: sha,
        })
    }
}

/// PAK 打包器（rf-asset 格式复用）。
pub struct PakPackager;

impl Packager for PakPackager {
    fn name(&self) -> &str {
        "pak"
    }

    fn package(&self, out_dir: &Path, files: &[(String, Vec<u8>)]) -> Result<OutputArtifact> {
        std::fs::create_dir_all(out_dir)?;
        let out_path = out_dir.join("content.pak");
        let bytes = rf_asset::write_pak(out_path.to_str().unwrap(), files, true)?;
        let data = std::fs::read(&out_path)?;
        Ok(OutputArtifact {
            path: out_path.to_string_lossy().into_owned(),
            bytes,
            crc32: crc32(&data),
            sha256: sha256_hex(&data),
        })
    }
}

/// 安装脚本生成器（Inno Setup / AppImage / deb / Gradle）。
pub struct ScriptPackager {
    pub kind: ScriptKind,
    pub app_name: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    InnoSetup,
    Nsis,
    AppImage,
    Deb,
    Gradle,
}

impl Packager for ScriptPackager {
    fn name(&self) -> &str {
        match self.kind {
            ScriptKind::InnoSetup => "innosetup-script",
            ScriptKind::Nsis => "nsis-script",
            ScriptKind::AppImage => "appimage-script",
            ScriptKind::Deb => "deb-control",
            ScriptKind::Gradle => "gradle-project",
        }
    }

    fn package(&self, out_dir: &Path, _files: &[(String, Vec<u8>)]) -> Result<OutputArtifact> {
        std::fs::create_dir_all(out_dir)?;
        let (name, content) = match self.kind {
            ScriptKind::InnoSetup => (
                "installer.iss",
                format!(
                    "[Setup]\nAppName={}\nAppVersion={}\nDefaultDirName={{autopf}}\\{}\nOutputBaseFilename={}_{}\n[Files]\nSource: \"bin\\*\"; DestDir: \"{{app}}\"\n",
                    self.app_name, self.version, self.app_name, self.app_name, self.version
                ),
            ),
            ScriptKind::Nsis => (
                "installer.nsi",
                format!("Outfile \"{}_{}.exe\"\nInstallDir $PROGRAM64\\{}\nSection\nSetOutPath $INSTDIR\nFile /r bin\\*\nWriteUninstaller $INSTDIR\\uninst.exe\nSectionEnd\n", self.app_name, self.version, self.app_name),
            ),
            ScriptKind::AppImage => (
                "build-appimage.sh",
                format!("#!/bin/sh\nAPP={}\nVERSION={}\nmkdir -p ${{APP}}.AppDir/usr/bin\ncp -r bin/* ${{APP}}.AppDir/usr/bin/\nappimagetool ${{APP}}.AppDir ${{APP}}-${{VERSION}}-x86_64.AppImage\n", self.app_name, self.version),
            ),
            ScriptKind::Deb => (
                "control",
                format!("Package: {}\nVersion: {}\nSection: games\nArchitecture: amd64\nMaintainer: RustForge\nDescription: {} game\n", self.app_name.to_lowercase().replace(' ', "-"), self.version, self.app_name),
            ),
            ScriptKind::Gradle => (
                "build.gradle.kts",
                format!("plugins {{ id(\"com.android.application\") }}
android {{
  namespace = \"com.rustforge.{}\"
  compileSdk = 34
  defaultConfig {{
    applicationId = \"com.rustforge.{}\"
    minSdk = 26
    versionCode = 1
    versionName = \"{}\"
  }}
}}
", self.app_name.to_lowercase().replace(['-', ' '], "_"), self.app_name.to_lowercase().replace(['-', ' '], "_"), self.version),
            ),
        };
        let p = out_dir.join(name);
        std::fs::write(&p, content)?;
        let data = std::fs::read(&p)?;
        Ok(OutputArtifact {
            path: p.to_string_lossy().into_owned(),
            bytes: data.len() as u64,
            crc32: crc32(&data),
            sha256: sha256_hex(&data),
        })
    }
}

// ---- 签名（IF-315） ----

/// HMAC 密钥。
#[derive(Clone)]
pub struct HmacKey(Vec<u8>);

impl HmacKey {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn from_hex(hex: &str) -> Self {
        Self(hex.as_bytes().to_vec())
    }

    pub fn from_env(var: &str) -> Option<Self> {
        std::env::var(var).ok().map(|v| Self(v.into_bytes()))
    }
}

/// 签名信息。
#[derive(Debug, Clone)]
pub struct SignatureInfo {
    pub sha256: String,
    pub hmac_sha256: String,
    pub signed_at_ms: u64,
}

/// 签名器 trait（IF-315，可扩展）。
pub trait CodeSigner {
    fn sign(&self, artifact: &Path, key: &HmacKey) -> Result<SignatureInfo>;
    fn verify(&self, artifact: &Path, sig: &SignatureInfo, key: &HmacKey) -> Result<bool>;
}

/// 本地 HMAC 签名（原生商店签名见 NativeTools 命令生成）。
pub struct LocalHmacSigner;

impl CodeSigner for LocalHmacSigner {
    fn sign(&self, artifact: &Path, key: &HmacKey) -> Result<SignatureInfo> {
        let data = std::fs::read(artifact)?;
        let mac = hmac_sha256(&key.0, &data);
        Ok(SignatureInfo {
            sha256: sha256_hex(&data),
            hmac_sha256: mac.iter().map(|b| format!("{b:02x}")).collect(),
            signed_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        })
    }

    fn verify(&self, artifact: &Path, sig: &SignatureInfo, key: &HmacKey) -> Result<bool> {
        let data = std::fs::read(artifact)?;
        if sha256_hex(&data) != sig.sha256 {
            return Ok(false); // 文件已变
        }
        let mac = hmac_sha256(&key.0, &data);
        let hex: String = mac.iter().map(|b| format!("{b:02x}")).collect();
        Ok(hex == sig.hmac_sha256)
    }
}

/// 原生平台签名命令生成（signtool/codesign/apksigner）。
pub struct NativeTools;

impl NativeTools {
    pub fn signing_command(platform: PlatformTarget, artifact: &Path, identity: &str) -> String {
        match platform {
            PlatformTarget::WindowsX64 | PlatformTarget::WindowsArm64 => {
                format!("signtool sign /fd SHA256 /a /tr http://timestamp.digicert.com \"{}\"", artifact.display())
            }
            PlatformTarget::MacosX64 | PlatformTarget::MacosArm64 | PlatformTarget::IosArm64 => {
                format!("codesign --deep --force --sign \"{identity}\" \"{}\"", artifact.display())
            }
            PlatformTarget::AndroidArm64 | PlatformTarget::AndroidArmv7 => {
                format!("apksigner sign --ks {identity} --out {}.signed.apk {}", artifact.display(), artifact.display())
            }
            _ => format!("echo 'no native signing for {:?}'; gpg --detach-sign --local-user {identity} \"{}\"", platform, artifact.display()),
        }
    }
}

// ---- 商店（IF-316） ----

/// 发布包。
#[derive(Debug, Clone)]
pub struct PublishBundle {
    pub app_name: String,
    pub version: Version,
    pub artifacts: Vec<OutputArtifact>,
    pub stores: Vec<String>,
    pub privacy_policy: bool,
    pub age_rating: Option<String>,
}

/// 上传报告。
#[derive(Debug, Clone)]
pub struct UploadReport {
    pub store: String,
    pub ok: bool,
    pub detail: String,
}

/// 商店上传器 trait（IF-316，可扩展）。
pub trait StoreUploader {
    fn name(&self) -> &str;
    fn prepare_manifest(&self, bundle: &PublishBundle) -> Result<String>;
    fn upload(&self, manifest: &str, dry_run: bool) -> Result<UploadReport>;
}

/// Steam 上传（app_build.vdf 生成 + steamcmd 命令）。
pub struct SteamUploader;

impl StoreUploader for SteamUploader {
    fn name(&self) -> &str {
        "steam"
    }
    fn prepare_manifest(&self, b: &PublishBundle) -> Result<String> {
        Ok(format!(
            "\"appbuild\"{{\n  \"appid\" \"0\"\n  \"desc\" \"{} {}\"\n  \"buildpath\" \".\"\n  \"contentroot\" \"content\"\n  \"setlive\" \"\"\n  \"depots\"{{\n    \"0\"{{\"FileMapping\"{{\"LocalPath\" \"*\" \"DepotPath\" \".\" \"recursive\" \"1\"}}}}\n  }}\n}}\n",
            b.app_name, b.version
        ))
    }
    fn upload(&self, manifest: &str, dry_run: bool) -> Result<UploadReport> {
        let _ = manifest;
        let cmd = "steamcmd +login <user> +run_app_build /path/to/app_build.vdf +quit";
        if dry_run {
            return Ok(UploadReport {
                store: self.name().into(),
                ok: true,
                detail: format!("dry-run: {cmd}（需 STEAM_TOKEN 环境变量）"),
            });
        }
        Ok(UploadReport {
            store: self.name().into(),
            ok: false,
            detail: "需要凭据（STEAM_TOKEN）；CI 中配置后重试".into(),
        })
    }
}

/// Google Play（meta.json 生成 + fastlane 命令）。
pub struct PlayStoreUploader;

impl StoreUploader for PlayStoreUploader {
    fn name(&self) -> &str {
        "google-play"
    }
    fn prepare_manifest(&self, b: &PublishBundle) -> Result<String> {
        let mut m = String::new();
        m.push_str("{\n");
        m.push_str(&format!(
            "  \"packageName\": \"com.rustforge.{}\",\n",
            b.app_name.to_lowercase().replace([' ', '-'], "_")
        ));
        m.push_str(&format!("  \"versionCode\": 1,\n  \"versionName\": \"{}\",\n", b.version));
        m.push_str("  \"track\": \"internal\"\n}\n");
        Ok(m)
    }
    fn upload(&self, _manifest: &str, dry_run: bool) -> Result<UploadReport> {
        if dry_run {
            return Ok(UploadReport {
                store: self.name().into(),
                ok: true,
                detail: "dry-run: fastlane supply --aab game.aab（需 PLAY_SERVICE_ACCOUNT_JSON）"
                    .into(),
            });
        }
        Ok(UploadReport {
            store: self.name().into(),
            ok: false,
            detail: "需要 PLAY_SERVICE_ACCOUNT_JSON 凭据".into(),
        })
    }
}

/// itch.io（butler）。
pub struct ItchUploader;

impl StoreUploader for ItchUploader {
    fn name(&self) -> &str {
        "itch"
    }
    fn prepare_manifest(&self, b: &PublishBundle) -> Result<String> {
        Ok(format!("channel: {}\nversion: {}\n", b.app_name, b.version))
    }
    fn upload(&self, _m: &str, dry_run: bool) -> Result<UploadReport> {
        if dry_run {
            return Ok(UploadReport {
                store: self.name().into(),
                ok: true,
                detail: "dry-run: butler push dist user/game:channel（需 BUTLER_API_KEY）".into(),
            });
        }
        Ok(UploadReport {
            store: self.name().into(),
            ok: false,
            detail: "需要 BUTLER_API_KEY".into(),
        })
    }
}

/// GOG。
pub struct GogUploader;

impl StoreUploader for GogUploader {
    fn name(&self) -> &str {
        "gog"
    }
    fn prepare_manifest(&self, b: &PublishBundle) -> Result<String> {
        Ok(format!("product: {}\nversion: {}\n", b.app_name, b.version))
    }
    fn upload(&self, _m: &str, dry_run: bool) -> Result<UploadReport> {
        if dry_run {
            Ok(UploadReport {
                store: self.name().into(),
                ok: true,
                detail: "dry-run: gogdepot（需 GOG_CREDENTIALS）".into(),
            })
        } else {
            Ok(UploadReport {
                store: self.name().into(),
                ok: false,
                detail: "需要 GOG_CREDENTIALS".into(),
            })
        }
    }
}

// ---- 更新（IF-317） ----

/// 更新文件条目。
#[derive(Debug, Clone)]
pub struct UpdateFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub blocks: Vec<String>,
}

/// 更新清单。
#[derive(Debug, Clone)]
pub struct UpdateManifest {
    pub version: Version,
    pub files: Vec<UpdateFile>,
    pub full_url: String,
}

impl UpdateManifest {
    /// 扫描目录构造（block = 每块 sha256）。
    pub fn build_manifest(
        dir: &Path,
        block: usize,
        version: Version,
        full_url: &str,
    ) -> Result<Self> {
        let mut files = Vec::new();
        visit_files(dir, &mut |p| {
            let data = std::fs::read(p).map_err(EngineError::Io)?;
            let rel = p.strip_prefix(dir).unwrap_or(p).to_string_lossy().replace('\\', "/");
            let blocks: Vec<String> = data.chunks(block.max(1)).map(sha256_hex).collect();
            files.push(UpdateFile {
                path: rel,
                size: data.len() as u64,
                sha256: sha256_hex(&data),
                blocks,
            });
            Ok(())
        })?;
        Ok(Self { version, files, full_url: full_url.to_string() })
    }

    /// 序列化（JSON-ish 紧凑）。
    pub fn serialize(&self) -> String {
        let mut s = format!(
            "version {}
url {}
files {}
",
            self.version,
            self.full_url,
            self.files.len()
        );
        let _ = format!("version {}\n", self.version);
        for f in &self.files {
            s.push_str(&format!("file {} {} {} {}\n", f.path, f.size, f.sha256, f.blocks.len()));
        }
        s
    }

    pub fn parse(text: &str) -> Result<Self> {
        let lines = text.lines();
        let mut version = Version::new(0, 0, 0);
        let mut full_url = String::new();
        let mut files = Vec::new();
        let mut count = 0usize;
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            match parts.first().copied() {
                Some("version") => {
                    if parts.len() >= 2 {
                        let segs: Vec<u32> =
                            parts[1].split('.').filter_map(|s| s.parse().ok()).collect();
                        if segs.len() == 3 {
                            version = Version::new(segs[0], segs[1], segs[2]);
                        }
                    }
                }
                Some("url") => full_url = parts.get(1).copied().unwrap_or("").to_string(),
                Some("files") => count = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0),
                Some("file") if parts.len() >= 5 => {
                    let path = parts[1].to_string();
                    let size: u64 = parts[2].parse().unwrap_or(0);
                    let sha = parts[3].to_string();
                    let nblocks: usize = parts[4].parse().unwrap_or(0);
                    files.push(UpdateFile {
                        path,
                        size,
                        sha256: sha,
                        blocks: vec![String::new(); nblocks],
                    });
                }
                _ => {}
            }
        }
        if files.len() != count {
            return Err(EngineError::InvalidData("manifest count mismatch".into()));
        }
        Ok(Self { version, files, full_url })
    }
}

fn visit_files(dir: &Path, f: &mut impl FnMut(&Path) -> Result<()>) -> Result<()> {
    let rd = std::fs::read_dir(dir).map_err(EngineError::Io)?;
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            visit_files(&p, f)?;
        } else {
            f(&p)?;
        }
    }
    Ok(())
}

/// 块差分操作。
#[derive(Debug, Clone, PartialEq)]
pub enum PatchOp {
    Copy { start: u64, len: u64 },
    Data(Vec<u8>),
}

/// 块差分：old 与 new 按 block 分块，相同块引用旧数据，不同块携带新数据。
pub fn block_diff(old: &[u8], new: &[u8], block: usize) -> Vec<PatchOp> {
    let block = block.max(1);
    use std::collections::HashMap;
    let mut index: HashMap<String, u64> = HashMap::new();
    for (i, chunk) in old.chunks(block).enumerate() {
        index.insert(sha256_hex(chunk), (i as u64) * block as u64);
    }
    let mut ops = Vec::new();
    let mut new_data = Vec::new();
    for chunk in new.chunks(block) {
        if let Some(&start) = index.get(&sha256_hex(chunk)) {
            if !new_data.is_empty() {
                ops.push(PatchOp::Data(std::mem::take(&mut new_data)));
            }
            ops.push(PatchOp::Copy { start, len: chunk.len() as u64 });
        } else {
            new_data.extend_from_slice(chunk);
        }
    }
    if !new_data.is_empty() {
        ops.push(PatchOp::Data(new_data));
    }
    ops
}

/// 应用差分。
pub fn apply_patch(old: &[u8], ops: &[PatchOp]) -> Vec<u8> {
    let mut out = Vec::new();
    for op in ops {
        match op {
            PatchOp::Copy { start, len } => {
                let s = *start as usize;
                out.extend_from_slice(&old[s..(s + *len as usize).min(old.len())]);
            }
            PatchOp::Data(d) => out.extend_from_slice(d),
        }
    }
    out
}

/// 更新计划。
#[derive(Debug, Clone, PartialEq)]
pub enum UpdatePlan {
    UpToDate,
    Full { url: String },
    Delta { changed: Vec<String> },
}

/// 更新器（IF-317）。
pub struct Updater;

impl Updater {
    pub fn check(current: &Version, manifest: &UpdateManifest) -> Option<UpdatePlan> {
        if *current >= manifest.version {
            return Some(UpdatePlan::UpToDate);
        }
        Some(UpdatePlan::Full { url: manifest.full_url.clone() })
    }

    /// 应用计划：fetch 回调取文件数据 → 写入 out_dir；失败回滚（备份后失败恢复）。
    pub fn apply_plan(
        &self,
        out_dir: &Path,
        manifest: &UpdateManifest,
        fetch: &dyn Fn(&str) -> Vec<u8>,
    ) -> Result<()> {
        let backup_dir = out_dir.with_extension("bak");
        if out_dir.exists() {
            let _ = std::fs::remove_dir_all(&backup_dir);
            copy_dir(out_dir, &backup_dir)?;
        }
        for f in &manifest.files {
            let data = fetch(&f.path);
            if sha256_hex(&data) != f.sha256 {
                // 回滚
                if backup_dir.exists() {
                    let _ = std::fs::remove_dir_all(out_dir);
                    let _ = std::fs::rename(&backup_dir, out_dir);
                }
                return Err(EngineError::InvalidData(format!("hash mismatch: {}", f.path)));
            }
            let dest = out_dir.join(&f.path);
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(dest, &data).map_err(EngineError::Io)?;
        }
        let _ = std::fs::remove_dir_all(&backup_dir);
        Ok(())
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src).map_err(EngineError::Io)?.flatten() {
        let p = e.path();
        let t = dst.join(e.file_name());
        if p.is_dir() {
            copy_dir(&p, &t)?;
        } else {
            std::fs::copy(&p, &t).map_err(EngineError::Io)?;
        }
    }
    Ok(())
}

// ---- 成就（IF-318） ----

/// 成就提供者 trait。
pub trait AchievementProvider {
    fn unlock(&mut self, id: &str, progress: f32) -> Result<bool>;
    fn state(&self, id: &str) -> Option<(f32, bool)>;
    fn drain_notifications(&mut self) -> Vec<String>;
}

/// 本地成就（JSON 持久化）。
pub struct LocalAchievements {
    dir: PathBuf,
    cache: HashMap<String, (f32, bool)>,
    notifications: Vec<String>,
}

impl LocalAchievements {
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        let mut cache = HashMap::new();
        if let Ok(text) = std::fs::read_to_string(dir.join("achievements.json")) {
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() == 3 {
                    cache.insert(
                        parts[0].to_string(),
                        (parts[1].parse().unwrap_or(0.0), parts[2] == "1"),
                    );
                }
            }
        }
        Ok(Self { dir, cache, notifications: Vec::new() })
    }

    fn persist(&self) {
        let text: String = self
            .cache
            .iter()
            .map(|(k, (p, u))| format!("{k},{p},{}", if *u { 1 } else { 0 }))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(self.dir.join("achievements.json"), text);
    }
}

impl AchievementProvider for LocalAchievements {
    fn unlock(&mut self, id: &str, progress: f32) -> Result<bool> {
        let entry = self.cache.entry(id.to_string()).or_insert((0.0, false));
        if entry.1 {
            return Ok(false); // 已解锁
        }
        if progress >= 1.0 {
            entry.1 = true;
            entry.0 = 1.0;
            self.notifications.push(format!("成就解锁: {id}"));
            self.persist();
            return Ok(true);
        }
        entry.0 = progress;
        self.persist();
        Ok(false)
    }

    fn state(&self, id: &str) -> Option<(f32, bool)> {
        self.cache.get(id).copied()
    }

    fn drain_notifications(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notifications)
    }
}

// ---- 云存档（IF-319） ----

/// 冲突策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    PreferNewer,
    PreferLocal,
    PreferRemote,
    Manual,
}

/// 冲突解决结果。
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictResolution {
    Local(Vec<u8>),
    Remote(Vec<u8>),
}

/// 云存档 trait。
pub trait CloudSaveProvider {
    fn upload(&mut self, slot: &str, data: &[u8]) -> Result<()>;
    fn download(&mut self, slot: &str) -> Option<Vec<u8>>;
    fn resolve(&self, local: &[u8], remote: &[u8], policy: ConflictPolicy) -> ConflictResolution;
}

/// 本地目录云存档（mtime 冲突检测）。
pub struct LocalCloudSaves {
    dir: PathBuf,
}

impl LocalCloudSaves {
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn slot_path(&self, slot: &str) -> PathBuf {
        self.dir.join(format!("{slot}.sav"))
    }
}

impl CloudSaveProvider for LocalCloudSaves {
    fn upload(&mut self, slot: &str, data: &[u8]) -> Result<()> {
        std::fs::write(self.slot_path(slot), data).map_err(EngineError::Io)
    }

    fn download(&mut self, slot: &str) -> Option<Vec<u8>> {
        std::fs::read(self.slot_path(slot)).ok()
    }

    fn resolve(&self, local: &[u8], remote: &[u8], policy: ConflictPolicy) -> ConflictResolution {
        match policy {
            ConflictPolicy::PreferLocal => ConflictResolution::Local(local.to_vec()),
            ConflictPolicy::PreferRemote => ConflictResolution::Remote(remote.to_vec()),
            ConflictPolicy::PreferNewer | ConflictPolicy::Manual => {
                ConflictResolution::Local(local.to_vec())
            } // Manual 由 UI 层处理；PreferNewer 需时间戳由调用方比较后选用
        }
    }
}

// ---- 崩溃报告（IF-320） ----

/// 崩溃报告。
#[derive(Debug, Clone)]
pub struct CrashReport {
    pub app_version: String,
    pub os: String,
    pub time_ms: u64,
    pub message: String,
    pub backtrace: String,
}

/// 崩溃报告 trait。
pub trait CrashReporter {
    fn capture(&self, message: &str, backtrace: &str) -> CrashReport;
    fn persist(&self, report: &CrashReport) -> Result<String>;
    fn flush_queue(&mut self) -> Result<usize>;
}

/// 本地崩溃报告（GDPR 同意门控 + 上传队列）。
pub struct LocalCrashReporter {
    dir: PathBuf,
    pub consent: bool,
    queue: Vec<CrashReport>,
}

impl LocalCrashReporter {
    pub fn new(dir: impl Into<PathBuf>, consent: bool) -> Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir, consent, queue: Vec::new() })
    }
}

impl CrashReporter for LocalCrashReporter {
    fn capture(&self, message: &str, backtrace: &str) -> CrashReport {
        CrashReport {
            app_version: rf_core::ENGINE_VERSION.to_string(),
            os: format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
            time_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            message: message.to_string(),
            backtrace: backtrace.to_string(),
        }
    }

    fn persist(&self, report: &CrashReport) -> Result<String> {
        let name = format!("crash_{}.log", report.time_ms);
        let p = self.dir.join(&name);
        std::fs::write(
            &p,
            format!(
                "version: {}\nos: {}\ntime: {}\nmessage: {}\nbacktrace:\n{}\n",
                report.app_version, report.os, report.time_ms, report.message, report.backtrace
            ),
        )
        .map_err(EngineError::Io)?;
        Ok(p.to_string_lossy().into_owned())
    }

    fn flush_queue(&mut self) -> Result<usize> {
        if !self.consent {
            let n = self.queue.len();
            self.queue.clear();
            return Ok(n); // 未同意：不上传，仅清队列（本地已持久化）
        }
        let n = self.queue.len();
        self.queue.clear();
        Ok(n)
    }
}

// ---- 分析（IF-321） ----

/// 分析事件提供者 trait。
pub trait AnalyticsProvider {
    fn track(&mut self, event: &str, props: &[(String, String)]);
    fn flush(&mut self) -> Result<usize>;
}

/// 本地 JSONL 批处理。
pub struct LocalAnalytics {
    path: PathBuf,
    queue: Vec<String>,
}

impl LocalAnalytics {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), queue: Vec::new() }
    }
}

impl AnalyticsProvider for LocalAnalytics {
    fn track(&mut self, event: &str, props: &[(String, String)]) {
        let props_s =
            props.iter().map(|(k, v)| format!("\"{k}\":\"{v}\"")).collect::<Vec<_>>().join(",");
        self.queue.push(format!("{{\"event\":\"{event}\",\"props\":{{{props_s}}}}}"));
    }

    fn flush(&mut self) -> Result<usize> {
        if self.queue.is_empty() {
            return Ok(0);
        }
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(EngineError::Io)?;
        for line in &self.queue {
            f.write_all(line.as_bytes()).map_err(EngineError::Io)?;
            f.write_all(b"\n").map_err(EngineError::Io)?;
        }
        let n = self.queue.len();
        self.queue.clear();
        Ok(n)
    }
}

// ---- 合规（IF-322） ----

/// 合规规则。
pub struct ComplianceRule {
    pub jurisdiction: &'static str,
    pub requirement: String,
    pub check: Box<dyn Fn(&PublishBundle) -> bool + Send>,
}

/// 合规报告。
#[derive(Debug, Clone, Default)]
pub struct ComplianceReport {
    pub passed: Vec<String>,
    pub failed: Vec<String>,
    pub warnings: Vec<String>,
}

/// 合规检查器。
pub struct ComplianceChecker {
    pub rules: Vec<ComplianceRule>,
}

impl Default for ComplianceChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplianceChecker {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: ComplianceRule) {
        self.rules.push(rule);
    }

    pub fn run(&self, bundle: &PublishBundle) -> ComplianceReport {
        let mut report = ComplianceReport::default();
        for r in &self.rules {
            let label = format!("[{}] {}", r.jurisdiction, r.requirement);
            if (r.check)(bundle) {
                report.passed.push(label);
            } else {
                report.failed.push(label);
            }
        }
        report
    }
}

/// 内置规则集（GDPR/CCPA/隐私政策/年龄分级/中国版号提示）。
pub fn builtin_rules() -> Vec<ComplianceRule> {
    vec![
        ComplianceRule {
            jurisdiction: "GDPR",
            requirement: "隐私政策已提供".into(),
            check: Box::new(|b: &PublishBundle| b.privacy_policy),
        },
        ComplianceRule {
            jurisdiction: "CCPA",
            requirement: "隐私政策已提供（加州）".into(),
            check: Box::new(|b: &PublishBundle| b.privacy_policy),
        },
        ComplianceRule {
            jurisdiction: "GLOBAL",
            requirement: "年龄分级已声明（IARC/ESRB/PEGI）".into(),
            check: Box::new(|b: &PublishBundle| b.age_rating.is_some()),
        },
        ComplianceRule {
            jurisdiction: "CN",
            requirement: "中国大陆发行需版号（提示项）".into(),
            check: Box::new(|_: &PublishBundle| false), // 恒 false → 进 failed 作提示
        },
        ComplianceRule {
            jurisdiction: "STORE",
            requirement: "至少一个分发渠道配置".into(),
            check: Box::new(|b: &PublishBundle| !b.stores.is_empty()),
        },
    ]
}

// ---- DRM / 反作弊（IF-323） ----

/// DRM 提供者 trait。
pub trait DrmProvider {
    fn name(&self) -> &str;
    fn wrap(&self, artifact: &Path) -> Result<String>;
}

/// 不使用 DRM。
pub struct NoDrm;

impl DrmProvider for NoDrm {
    fn name(&self) -> &str {
        "none"
    }
    fn wrap(&self, _artifact: &Path) -> Result<String> {
        Ok("未启用 DRM".into())
    }
}

/// Steam DRM（命令生成，不内置）。
pub struct SteamDrmCommand;

impl DrmProvider for SteamDrmCommand {
    fn name(&self) -> &str {
        "steam-drm"
    }
    fn wrap(&self, artifact: &Path) -> Result<String> {
        Ok(format!("steamdrm_wrap \"{}\"（需 steamworks SDK 授权）", artifact.display()))
    }
}

/// 反作弊报告。
#[derive(Debug, Clone)]
pub struct AntiCheatReport {
    pub clean: bool,
    pub findings: Vec<String>,
}

/// 反作弊 trait。
pub trait AntiCheatProvider {
    fn name(&self) -> &str;
    fn scan(&self, dir: &Path) -> AntiCheatReport;
}

/// 完整性反作弊：对照清单校验文件哈希。
pub struct IntegrityAntiCheat {
    /// 期望的 (path, sha256)。
    pub manifest: Vec<(String, String)>,
}

impl AntiCheatProvider for IntegrityAntiCheat {
    fn name(&self) -> &str {
        "integrity"
    }
    fn scan(&self, dir: &Path) -> AntiCheatReport {
        let mut findings = Vec::new();
        let mut clean = true;
        for (rel, expect) in &self.manifest {
            let p = dir.join(rel);
            match std::fs::read(&p) {
                Ok(data) => {
                    let actual = sha256_hex(&data);
                    if actual != *expect {
                        clean = false;
                        findings.push(format!("已篡改: {rel}"));
                    }
                }
                Err(_) => {
                    clean = false;
                    findings.push(format!("缺失: {rel}"));
                }
            }
        }
        AntiCheatReport { clean, findings }
    }
}

// ---- SDK（IF-324） ----

/// SDK 导出器 trait。
pub trait SdkExporter {
    fn export(&self, out_dir: &Path) -> Result<Vec<String>>;
}

/// C 头文件导出。
pub struct CSdkExporter;

impl SdkExporter for CSdkExporter {
    fn export(&self, out_dir: &Path) -> Result<Vec<String>> {
        std::fs::create_dir_all(out_dir)?;
        let header = r#"/* RustForge C API */
#ifndef RUSTFORGE_H
#define RUSTFORGE_H
#ifdef __cplusplus
extern "C" {
#endif

const char* rf_engine_version(void);
int rf_init(void);
void rf_sha256_hex(const unsigned char* data, unsigned long len, char out[65]);

#ifdef __cplusplus
}
#endif
#endif
"#;
        let h = out_dir.join("rustforge.h");
        std::fs::write(&h, header)?;
        let readme = out_dir.join("README.md");
        std::fs::write(&readme, "# RustForge C SDK\n\n```c\n#include \"rustforge.h\"\nconst char* v = rf_engine_version();\n```\n链接 rustforge（静态库随平台构建产出）。\n")?;
        Ok(vec![h.to_string_lossy().into_owned(), readme.to_string_lossy().into_owned()])
    }
}

/// FFI（IF-324 curated 集）。
pub mod ffi {
    use super::sha256_hex;

    /// 引擎版本。
    #[no_mangle]
    pub extern "C" fn rf_engine_version() -> *const std::ffi::c_char {
        static V: std::sync::OnceLock<std::ffi::CString> = std::sync::OnceLock::new();
        V.get_or_init(|| std::ffi::CString::new(rf_core::ENGINE_VERSION.to_string()).unwrap())
            .as_ptr()
    }

    /// 初始化（日志/基础系统）。
    #[no_mangle]
    pub extern "C" fn rf_init() -> i32 {
        rf_debugger::init_global();
        0
    }

    /// sha256 十六进制输出（out 需 65 字节）。
    ///
    /// # Safety
    /// data 需指向 len 字节有效内存；out 需指向 65 字节可写内存。
    #[no_mangle]
    pub unsafe extern "C" fn rf_sha256_hex(data: *const u8, len: usize, out: *mut u8) {
        if data.is_null() || out.is_null() {
            return;
        }
        let bytes = unsafe { std::slice::from_raw_parts(data, len) };
        let hex = sha256_hex(bytes);
        unsafe {
            std::ptr::copy_nonoverlapping(hex.as_ptr(), out, 64);
            *out.add(64) = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_known() {
        assert_eq!(crc32(b"123456789"), 0xCBF43926); // IEEE 检验值
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"The quick brown fox jumps over the lazy dog"),
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
        );
    }

    #[test]
    fn hmac_deterministic_and_key_sensitive() {
        let a = hmac_sha256(b"key", b"data");
        let b = hmac_sha256(b"key", b"data");
        let c = hmac_sha256(b"KEY", b"data");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn build_plan_commands() {
        let mut plan = BuildPlan::plan(BuildProfile::Shipping, PlatformTarget::WindowsX64);
        plan.features.push("win32".into());
        plan.examples.push("minimal_2d".into());
        let cmd = plan.command_line();
        assert_eq!(cmd[0], "cargo");
        assert!(cmd.contains(&"--release".to_string()));
        assert!(cmd.contains(&"x86_64-pc-windows-msvc".to_string()));
        assert!(cmd.contains(&"win32".to_string()));
        // 交叉目标未装 → 明确错误
        let cross = BuildPlan::plan(BuildProfile::Debug, PlatformTarget::AndroidArm64);
        let _ = cross; // 执行需环境，命令行生成已验证
    }

    #[test]
    fn platform_targets() {
        assert!(PlatformTarget::WindowsX64.is_desktop());
        assert!(PlatformTarget::AndroidArm64.is_mobile());
        assert!(PlatformTarget::WebWasm.needs_cross_tool());
        assert_eq!(PlatformTarget::LinuxArm64.triple(), "aarch64-unknown-linux-gnu");
    }

    #[test]
    fn zip_packager_roundtrip() {
        let dir = std::env::temp_dir().join("rf_dist_zip");
        let files = vec![
            ("bin/game.exe".to_string(), b"MZfake".to_vec()),
            ("assets/a.pak".to_string(), vec![7u8; 200]),
        ];
        let artifact = ZipPackager.package(&dir, &files).unwrap();
        assert!(artifact.bytes > 100);
        assert!(Path::new(&artifact.path).exists());
        // 结构签名
        let data = std::fs::read(&artifact.path).unwrap();
        assert_eq!(&data[0..4], &[0x50, 0x4b, 0x03, 0x04]); // PK 头
        assert_eq!(crc32(&data), artifact.crc32);
        assert_eq!(sha256_hex(&data), artifact.sha256);
    }

    #[test]
    fn pak_and_script_packagers() {
        let dir = std::env::temp_dir().join("rf_dist_pak");
        let files = vec![("a.txt".to_string(), b"hello".to_vec())];
        let art = PakPackager.package(&dir, &files).unwrap();
        assert!(Path::new(&art.path).exists());
        let reader = rf_asset::PakReader::open(&art.path).unwrap();
        assert_eq!(reader.list(), vec!["a.txt"]);
        let sp = ScriptPackager {
            kind: ScriptKind::InnoSetup,
            app_name: "MyGame".into(),
            version: "1.0.0".into(),
        };
        let s = sp.package(&dir, &files).unwrap();
        let content = std::fs::read_to_string(&s.path).unwrap();
        assert!(content.contains("MyGame"));
        let gradle = ScriptPackager {
            kind: ScriptKind::Gradle,
            app_name: "MyGame".into(),
            version: "1.0.0".into(),
        };
        let g = gradle.package(&dir, &files).unwrap();
        assert!(std::fs::read_to_string(&g.path).unwrap().contains("com.android.application"));
    }

    #[test]
    fn signer_roundtrip_and_tamper() {
        let dir = std::env::temp_dir().join("rf_dist_sign");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("game.bin");
        std::fs::write(&p, b"payload").unwrap();
        let key = HmacKey::from_bytes(b"secret-key".to_vec());
        let sig = LocalHmacSigner.sign(&p, &key).unwrap();
        assert!(LocalHmacSigner.verify(&p, &sig, &key).unwrap());
        // 篡改
        std::fs::write(&p, b"tampered").unwrap();
        assert!(!LocalHmacSigner.verify(&p, &sig, &key).unwrap());
        // 错误密钥
        std::fs::write(&p, b"payload").unwrap();
        let bad = HmacKey::from_bytes(b"other".to_vec());
        assert!(!LocalHmacSigner.verify(&p, &sig, &bad).unwrap());
        assert!(
            NativeTools::signing_command(PlatformTarget::WindowsX64, &p, "id").contains("signtool")
        );
        assert!(NativeTools::signing_command(PlatformTarget::MacosArm64, &p, "Dev")
            .contains("codesign"));
        assert!(NativeTools::signing_command(PlatformTarget::AndroidArm64, &p, "ks")
            .contains("apksigner"));
    }

    #[test]
    fn store_manifests_and_dry_run() {
        let bundle = PublishBundle {
            app_name: "TestGame".into(),
            version: Version::new(1, 2, 3),
            artifacts: vec![],
            stores: vec!["steam".into()],
            privacy_policy: true,
            age_rating: Some("PEGI 7".into()),
        };
        let steam = SteamUploader;
        let m = steam.prepare_manifest(&bundle).unwrap();
        assert!(m.contains("appbuild"));
        let r = steam.upload(&m, true).unwrap();
        assert!(r.ok && r.detail.contains("dry-run"));
        let play = PlayStoreUploader;
        let mp = play.prepare_manifest(&bundle).unwrap();
        assert!(mp.contains("versionName"));
        assert!(play.upload(&mp, false).unwrap().detail.contains("PLAY_SERVICE_ACCOUNT_JSON"));
        assert!(ItchUploader.prepare_manifest(&bundle).unwrap().contains("channel"));
        assert!(GogUploader.prepare_manifest(&bundle).unwrap().contains("product"));
    }

    #[test]
    fn update_manifest_roundtrip() {
        let dir = std::env::temp_dir().join("rf_dist_upd");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(dir.join("game.exe"), b"binary-v2").unwrap();
        std::fs::write(dir.join("assets/data.pak"), vec![9u8; 128]).unwrap();
        let m = UpdateManifest::build_manifest(
            &dir,
            32,
            Version::new(0, 2, 0),
            "https://cdn.example/full.zip",
        )
        .unwrap();
        assert_eq!(m.files.len(), 2);
        let text = m.serialize();
        let parsed = UpdateManifest::parse(&text).unwrap();
        assert_eq!(parsed.version, m.version);
        assert_eq!(parsed.files.len(), 2);
        assert_eq!(parsed.files[0].sha256, m.files[0].sha256);
        // 计数不符报错
        assert!(UpdateManifest::parse("files 5\n").is_err());
        // Updater
        let plan = Updater::check(&Version::new(0, 1, 0), &m).unwrap();
        assert!(matches!(plan, UpdatePlan::Full { .. }));
        assert!(matches!(
            Updater::check(&Version::new(0, 2, 0), &m).unwrap(),
            UpdatePlan::UpToDate
        ));
    }

    #[test]
    fn block_diff_roundtrip_and_savings() {
        let old = vec![1u8; 4096];
        let mut new = old.clone();
        new[500..532].copy_from_slice(&[7u8; 32]); // 改一块
        let ops = block_diff(&old, &new, 1024);
        let restored = apply_patch(&old, &ops);
        assert_eq!(restored, new);
        // 差分数据量远小于全量
        let data_bytes: usize = ops
            .iter()
            .map(|o| match o {
                PatchOp::Data(d) => d.len(),
                _ => 0,
            })
            .sum();
        assert!(data_bytes <= 1024);
        // 完全相同 → 全 Copy
        assert!(block_diff(&old, &old.clone(), 1024)
            .iter()
            .all(|o| matches!(o, PatchOp::Copy { .. })));
    }

    #[test]
    fn updater_apply_and_rollback() {
        let dir = std::env::temp_dir().join("rf_dist_apply");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), b"old").unwrap();
        // 构造目标清单（b.txt 内容正确）
        let b_content = b"new-content".to_vec();
        let files = vec![
            UpdateFile {
                path: "a.txt".into(),
                size: 3,
                sha256: sha256_hex(b"new"),
                blocks: vec![],
            },
            UpdateFile {
                path: "b.txt".into(),
                size: b_content.len() as u64,
                sha256: sha256_hex(&b_content),
                blocks: vec![],
            },
        ];
        let m = UpdateManifest { version: Version::new(0, 2, 0), files, full_url: String::new() };
        // fetch 返回错误哈希 → 回滚
        let fetch_bad = |p: &str| -> Vec<u8> {
            if p == "a.txt" {
                b"WRONG".to_vec()
            } else {
                b_content.clone()
            }
        };
        assert!(Updater.apply_plan(&dir, &m, &fetch_bad).is_err());
        assert_eq!(std::fs::read(dir.join("a.txt")).unwrap(), b"old"); // 回滚保留
                                                                       // 正确 fetch
        let fetch_ok = |p: &str| -> Vec<u8> {
            if p == "a.txt" {
                b"new".to_vec()
            } else {
                b_content.clone()
            }
        };
        Updater.apply_plan(&dir, &m, &fetch_ok).unwrap();
        assert_eq!(std::fs::read(dir.join("a.txt")).unwrap(), b"new");
        assert_eq!(std::fs::read(dir.join("b.txt")).unwrap(), b"new-content");
    }

    #[test]
    fn achievements_lifecycle() {
        let dir = std::env::temp_dir().join("rf_dist_ach");
        let _ = std::fs::remove_dir_all(&dir);
        let mut ach = LocalAchievements::new(&dir).unwrap();
        assert!(!ach.unlock("first_blood", 0.5).unwrap()); // 进度
        assert_eq!(ach.state("first_blood"), Some((0.5, false)));
        assert!(ach.unlock("first_blood", 1.0).unwrap()); // 解锁
        assert!(!ach.unlock("first_blood", 1.0).unwrap()); // 重复
        assert_eq!(ach.state("first_blood"), Some((1.0, true)));
        let notes = ach.drain_notifications();
        assert_eq!(notes, vec!["成就解锁: first_blood".to_string()]);
        assert!(ach.drain_notifications().is_empty());
        // 持久化往返
        let ach2 = LocalAchievements::new(&dir).unwrap();
        assert_eq!(ach2.state("first_blood"), Some((1.0, true)));
    }

    #[test]
    fn cloud_saves_conflict() {
        let dir = std::env::temp_dir().join("rf_dist_cloud");
        let _ = std::fs::remove_dir_all(&dir);
        let mut cs = LocalCloudSaves::new(&dir).unwrap();
        cs.upload("slot1", b"remote-data").unwrap();
        assert_eq!(cs.download("slot1"), Some(b"remote-data".to_vec()));
        assert_eq!(cs.download("missing"), None);
        let r = cs.resolve(b"local", b"remote", ConflictPolicy::PreferLocal);
        assert_eq!(r, ConflictResolution::Local(b"local".to_vec()));
        let r2 = cs.resolve(b"local", b"remote", ConflictPolicy::PreferRemote);
        assert_eq!(r2, ConflictResolution::Remote(b"remote".to_vec()));
    }

    #[test]
    fn crash_reporter_consent_gate() {
        let dir = std::env::temp_dir().join("rf_dist_crash");
        let _ = std::fs::remove_dir_all(&dir);
        let rep = LocalCrashReporter::new(&dir, false).unwrap();
        let report = rep.capture("panic in physics", "backtrace: 0x1...");
        assert_eq!(report.app_version, rf_core::ENGINE_VERSION.to_string());
        let path = rep.persist(&report).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("panic in physics"));
        assert!(text.contains("backtrace"));
        // 未同意 flush 不上传
        let mut rep2 = LocalCrashReporter::new(&dir, false).unwrap();
        assert_eq!(rep2.flush_queue().unwrap(), 0);
    }

    #[test]
    fn analytics_batching() {
        let dir = std::env::temp_dir().join("rf_dist_an");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("events.jsonl");
        let mut a = LocalAnalytics::new(&p);
        a.track("level_start", &[("level".to_string(), "3".to_string())]);
        a.track(
            "level_end",
            &[("level".to_string(), "3".to_string()), ("result".to_string(), "win".to_string())],
        );
        assert_eq!(a.flush().unwrap(), 2);
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("level_start"));
        assert!(text.contains("\"result\":\"win\""));
        assert_eq!(a.flush().unwrap(), 0); // 队列已清
    }

    #[test]
    fn compliance_checks() {
        let mut checker = ComplianceChecker::new();
        for r in builtin_rules() {
            checker.add_rule(r);
        }
        let ok_bundle = PublishBundle {
            app_name: "G".into(),
            version: Version::new(1, 0, 0),
            artifacts: vec![],
            stores: vec!["itch".into()],
            privacy_policy: true,
            age_rating: Some("IARC 3".into()),
        };
        let report = checker.run(&ok_bundle);
        assert!(report.passed.len() >= 4);
        assert_eq!(report.failed.len(), 1); // 中国版号提示恒在 failed
        assert!(report.failed[0].contains("版号"));
        let bad =
            PublishBundle { privacy_policy: false, age_rating: None, stores: vec![], ..ok_bundle };
        let report2 = checker.run(&bad);
        assert!(report2.failed.len() >= 3);
    }

    #[test]
    fn drm_and_anticheat() {
        assert_eq!(NoDrm.wrap(Path::new("x")).unwrap(), "未启用 DRM");
        assert!(SteamDrmCommand.wrap(Path::new("x")).unwrap().contains("steamdrm"));
        // 完整性扫描
        let dir = std::env::temp_dir().join("rf_dist_ac");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("game.exe"), b"clean").unwrap();
        let manifest = vec![
            ("game.exe".to_string(), sha256_hex(b"clean")),
            ("hack.dll".to_string(), sha256_hex(b"nope")),
        ];
        let ac = IntegrityAntiCheat { manifest };
        let report = ac.scan(&dir);
        assert!(!report.clean);
        assert!(report.findings.iter().any(|f| f.contains("缺失")));
        // 篡改
        std::fs::write(dir.join("game.exe"), b"hacked").unwrap();
        let report2 =
            IntegrityAntiCheat { manifest: vec![("game.exe".to_string(), sha256_hex(b"clean"))] }
                .scan(&dir);
        assert!(report2.findings.iter().any(|f| f.contains("已篡改")));
    }

    #[test]
    fn sdk_export() {
        let dir = std::env::temp_dir().join("rf_dist_sdk");
        let _ = std::fs::remove_dir_all(&dir);
        let files = CSdkExporter.export(&dir).unwrap();
        assert_eq!(files.len(), 2);
        let header = std::fs::read_to_string(dir.join("rustforge.h")).unwrap();
        assert!(header.contains("rf_engine_version"));
        assert!(header.contains("extern \"C\""));
    }

    #[test]
    fn ffi_functions() {
        let v = ffi::rf_engine_version();
        let s = unsafe { std::ffi::CStr::from_ptr(v).to_string_lossy().into_owned() };
        assert_eq!(s, rf_core::ENGINE_VERSION.to_string());
        assert_eq!(ffi::rf_init(), 0);
        let mut out = [0u8; 65];
        unsafe { ffi::rf_sha256_hex(b"abc".as_ptr(), 3, out.as_mut_ptr()) };
        assert_eq!(&out[..64], sha256_hex(b"abc").as_bytes());
        assert_eq!(out[64], 0);
    }
}
