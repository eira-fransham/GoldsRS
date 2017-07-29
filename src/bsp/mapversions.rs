use sys::bsp::{Quake1Lump, Quake2Lump};

pub struct Quake1;
pub struct Quake2;
pub struct Goldsrc;

pub trait MapVersion {
    type Magic;
    type Lump;

    fn accepts_version(version: u32) -> bool;
}

impl MapVersion for Quake1 {
    type Magic = ();
    type Lump = Quake1Lump;

    fn accepts_version(version: u32) -> bool {
        version <= 0x1d
    }
}

impl MapVersion for Goldsrc {
    type Magic = ();
    type Lump = Quake1Lump;

    fn accepts_version(version: u32) -> bool {
        version == 0x1e
    }
}

impl MapVersion for Quake2 {
    type Magic = [u8; 4];
    type Lump = Quake2Lump;

    fn accepts_version(version: u32) -> bool {
        version <= 0x26 && version > 0x1d
    }
}
