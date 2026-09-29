//! CPU 侧 RGBA8 图像（IF-009）：Software RHI 帧缓冲、资产纹理、平台呈现共用。

/// 行主序、紧密排列的 RGBA8（每像素 4 字节）图像。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Rgba8Image {
    /// 全黑透明图像。
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height, data: vec![0; (width as usize) * (height as usize) * 4] }
    }

    /// 以颜色填充。
    pub fn filled(width: u32, height: u32, rgba: [u8; 4]) -> Self {
        let mut img = Self::new(width, height);
        for px in img.data.chunks_exact_mut(4) {
            px.copy_from_slice(&rgba);
        }
        img
    }

    /// 从原始字节构造（长度必须等于 w*h*4），越界/不足返回错误。
    pub fn from_raw(width: u32, height: u32, data: Vec<u8>) -> crate::Result<Self> {
        let expect = width as usize * height as usize * 4;
        if data.len() != expect {
            return Err(crate::EngineError::InvalidData(format!(
                "image raw len {} != {} ({}x{}x4)",
                data.len(),
                expect,
                width,
                height
            )));
        }
        Ok(Self { width, height, data })
    }

    pub fn with_mut<R>(&mut self, f: impl FnOnce(&mut [u8]) -> R) -> R {
        f(&mut self.data)
    }

    /// 越界返回 None。
    pub fn get(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = ((y as usize * self.width as usize) + x as usize) * 4;
        Some([self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]])
    }

    /// 越界忽略。
    pub fn set(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = ((y as usize * self.width as usize) + x as usize) * 4;
        self.data[i..i + 4].copy_from_slice(&rgba);
    }

    pub fn clear(&mut self, rgba: [u8; 4]) {
        for px in self.data.chunks_exact_mut(4) {
            px.copy_from_slice(&rgba);
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_basics() {
        let mut img = Rgba8Image::new(4, 4);
        img.set(1, 2, [255, 0, 0, 255]);
        assert_eq!(img.get(1, 2), Some([255, 0, 0, 255]));
        assert_eq!(img.get(4, 0), None);
        img.set(99, 99, [1, 1, 1, 1]); // 越界忽略
        let f = Rgba8Image::filled(2, 3, [9, 8, 7, 6]);
        assert_eq!(f.get(1, 2), Some([9, 8, 7, 6]));
        assert!(Rgba8Image::from_raw(2, 2, vec![0; 15]).is_err());
        assert!(Rgba8Image::from_raw(2, 2, vec![0; 16]).is_ok());
    }
}
