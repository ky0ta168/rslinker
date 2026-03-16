/// バイナリファイルの読み書きユーティリティ。
/// C++版の BinaryFile に相当する。
/// std::io::Read/Write/Seek を実装した任意の型に対して動作する。
use std::io::{Read, Seek, SeekFrom, Write};

use crate::error::Result;

// ---------------------------------------------------------------------------
// 読み込み
// ---------------------------------------------------------------------------

pub trait ReadExt: Read + Seek {
    fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16_le(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u32_le(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u64_le(&mut self) -> Result<u64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut buf = [0u8; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn position(&mut self) -> Result<u64> {
        Ok(self.stream_position()?)
    }

    fn set_position(&mut self, pos: u64) -> Result<()> {
        self.seek(SeekFrom::Start(pos))?;
        Ok(())
    }
}

impl<T: Read + Seek> ReadExt for T {}

// ---------------------------------------------------------------------------
// 書き込み
// ---------------------------------------------------------------------------

pub trait WriteExt: Write {
    fn write_u8(&mut self, v: u8) -> Result<()> {
        self.write_all(&[v])?;
        Ok(())
    }

    fn write_u16_le(&mut self, v: u16) -> Result<()> {
        self.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    fn write_u32_le(&mut self, v: u32) -> Result<()> {
        self.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    fn write_u64_le(&mut self, v: u64) -> Result<()> {
        self.write_all(&v.to_le_bytes())?;
        Ok(())
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.write_all(bytes)?;
        Ok(())
    }

    /// 現在位置を `target` までゼロパディングで埋める。
    fn pad_to(&mut self, current: u64, target: u64) -> Result<()> {
        if target > current {
            let zeros = vec![0u8; (target - current) as usize];
            self.write_all(&zeros)?;
        }
        Ok(())
    }
}

impl<T: Write> WriteExt for T {}

// ---------------------------------------------------------------------------
// ユーティリティ
// ---------------------------------------------------------------------------

/// `size` を `align` の倍数に切り上げる。align は 2 のべき乗を想定。
pub fn align_up(size: u32, align: u32) -> u32 {
    assert!(align > 0);
    (size + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_write_roundtrip() {
        let mut buf = Cursor::new(Vec::new());
        buf.write_u32_le(0xDEAD_BEEF).unwrap();
        buf.set_position(0);
        assert_eq!(buf.read_u32_le().unwrap(), 0xDEAD_BEEF);
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 0x200), 0);
        assert_eq!(align_up(1, 0x200), 0x200);
        assert_eq!(align_up(0x200, 0x200), 0x200);
        assert_eq!(align_up(0x201, 0x200), 0x400);
        assert_eq!(align_up(0x1000, 0x1000), 0x1000);
    }
}
