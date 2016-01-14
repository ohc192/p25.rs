use collect_slice::CollectSlice;

use bits::{Dibit, Hexbit, HexbitBytes};
use buffer::{Buffer, DibitStorage, VoiceHeaderStorage, VoiceExtraStorage};
use coding::{reed_solomon, golay};
use error::Result;

use voice::header::VoiceHeaderDecoder;
use voice::control::LinkControl;

use error::P25Error::*;

pub struct VoiceHeaderReceiver {
    dibits: Buffer<DibitStorage>,
    hexbits: Buffer<VoiceHeaderStorage>,
}

impl VoiceHeaderReceiver {
    pub fn new() -> VoiceHeaderReceiver {
        VoiceHeaderReceiver {
            dibits: Buffer::new(DibitStorage::new(9)),
            hexbits: Buffer::new(VoiceHeaderStorage::new()),
        }
    }

    pub fn feed(&mut self, dibit: Dibit) -> Option<Result<VoiceHeaderDecoder>> {
        let buf = match self.dibits.feed(dibit) {
            Some(buf) => *buf as u32,
            None => return None,
        };

        let data = match golay::shortened::decode(buf) {
            Some((data, err)) => data,
            None => return Some(Err(GolayUnrecoverable)),
        };

        let hexbits = match self.hexbits.feed(Hexbit::new(data)) {
            Some(buf) => buf,
            None => return None,
        };

        let data = match reed_solomon::long::decode(hexbits) {
            Some((data, err)) => data,
            None => return Some(Err(ReedSolomonUnrecoverable)),
        };

        let mut bytes = [0; 15];
        HexbitBytes::new(data.iter().cloned())
            .collect_slice_checked(&mut bytes[..]);

        Some(Ok(VoiceHeaderDecoder::new(bytes)))
    }
}

pub struct VoiceLCTerminatorReceiver {
    outer: Buffer<DibitStorage>,
    inner: Buffer<VoiceExtraStorage>,
}

impl VoiceLCTerminatorReceiver {
    pub fn new() -> VoiceLCTerminatorReceiver {
        VoiceLCTerminatorReceiver {
            outer: Buffer::new(DibitStorage::new(12)),
            inner: Buffer::new(VoiceExtraStorage::new()),
        }
    }

    pub fn feed(&mut self, dibit: Dibit) -> Option<Result<LinkControl>> {
        let buf = match self.outer.feed(dibit) {
            Some(buf) => buf,
            None => return None,
        };

        let data = match golay::extended::decode(*buf as u32) {
            Some((data, err)) => data,
            None => return Some(Err(GolayUnrecoverable)),
        };

        assert!(self.inner.feed(Hexbit::new((data >> 6) as u8)).is_none());

        let hexbits = match self.inner.feed(Hexbit::new((data & 0x3F) as u8)) {
            Some(buf) => buf,
            None => return None,
        };

        let data = match reed_solomon::short::decode(hexbits) {
            Some((data, err)) => data,
            None => return Some(Err(ReedSolomonUnrecoverable)),
        };

        let mut bytes = [0; 9];
        HexbitBytes::new(data.iter().cloned())
            .collect_slice_checked(&mut bytes[..]);

        Some(Ok(LinkControl::new(bytes)))
    }
}
