//! BSP loading code
//!
//! All numeric types use uNN/iNN types instead of `libc::c_****` types because Quake assumes that
//! it's compiled for a 32-bit computer. These numbers are all used for file IO and therefore do not
//! respect the actual C type of the numbers used.

use ioendian::{Little, IntoNativeEndian};
use std::marker::PhantomData;

type LU8 = Little<u8>;
type LU16 = Little<u16>;
type LU32 = Little<u32>;
type LI8 = Little<i8>;
type LI16 = Little<i16>;
type LI32 = Little<i32>;
type LF32 = Little<f32>;

pub type Scalar = LF32;
pub type BBoxV3 = BoundingBox<Scalar3>;
pub type BBoxShort = BoundingBox<Short3>;

#[derive(Debug, Clone, Copy)]
pub enum Unimplemented {}
pub trait UnifiesWith<T> {}

#[cfg(feature = "nightly")]
impl<T> UnifiesWith<T> for T {}
#[cfg(feature = "nightly")]
impl<T> UnifiesWith<T> for Unimplemented {}

#[cfg(not(feature = "nightly"))]
impl<T> UnifiesWith<T> for T {}

#[repr(C)]
#[derive(Debug)]
pub struct Entry<T = Unimplemented> {
    pub offset: LI32,
    pub len: LI32,
    pub output: PhantomData<T>,
}

impl<T> Clone for Entry<T> {
    fn clone(&self) -> Self {
        Entry {
            offset: self.offset,
            len: self.len,
            output: PhantomData,
        }
    }
}

impl<T> Entry<T> {
    pub fn transmute<U>(self) -> Entry<U> {
        Entry {
            offset: self.offset,
            len: self.len,
            output: PhantomData,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Header<M, L> {
    pub magic: M,
    pub version: LU32,
    pub lumps: L,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Quake1Lump {
    pub entities: Entry,
    pub planes: Entry<Plane>,
    pub miptex: Entry,
    pub vertices: Entry<Scalar3>,
    pub vislist: Entry<u8>,
    pub nodes: Entry<Node>,
    pub texinfo: Entry,
    pub faces: Entry<Face>,
    pub lightmaps: Entry,
    pub clipnodes: Entry,
    pub leaves: Entry<Leaf>,
    pub lfaces: Entry<LU16>,
    pub edges: Entry<Edge>,
    pub ledges: Entry<LI16>,
    pub models: Entry<Model>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Quake2Lump {
    pub entities: Entry,
    pub planes: Entry<Plane>,
    pub vertices: Entry<Scalar3>,
    pub vislist: Entry<u8>,
    pub nodes: Entry<Node>,
    pub texinfo: Entry,
    pub faces: Entry<Face>,
    pub lightmaps: Entry,
    pub leaves: Entry<Leaf>,
    pub lface: Entry<LU16>,
    pub lbrush: Entry,
    pub edges: Entry<Edge>,
    pub ledges: Entry<LI16>,
    pub models: Entry<Model>,
    pub brushes: Entry,
    pub brush_sides: Entry,
    pub pop: Entry,
    pub areas: Entry,
    pub area_portals: Entry,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
// TODO: Use nalgebra
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T: IntoNativeEndian> Vec3<T> {
    pub fn native(self) -> Vec3<T::Out> {
        Vec3 {
            x: self.x.native(),
            y: self.y.native(),
            z: self.z.native(),
        }
    }
}

pub type Scalar3 = Vec3<Scalar>;
pub type Short3 = Vec3<LI16>;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct BoundingBox<T> {
    pub aa: T,
    pub bb: T,
}

impl<T: IntoNativeEndian> BoundingBox<T> {
    pub fn native(self) -> BoundingBox<T::Out> {
        BoundingBox {
            aa: self.aa.native(),
            bb: self.bb.native(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Model {
    pub bound: BBoxV3,
    pub origin: Scalar3,
    pub hulls: [LI32; 4],
    pub numleafs: LI32,
    pub face_id: LI32,
    pub face_len: LI32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub start: LU16,
    pub end: LU16,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct TextureCoord {
    pub vector: Scalar3,
    pub distance: Scalar,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Surface {
    pub s: TextureCoord,
    pub t: TextureCoord,
    pub texture: LU32,
    pub animated: LU32, // Actually represents a bool
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Face {
    pub plane_id: LU16,
    pub side: LU16,
    pub ledge_id: LU32,
    pub ledge_len: LU32,
    pub texinfo_id: LU16,
    pub typelight: LU8,
    pub baselight: LU8,
    pub light: [LU8; 2],
    pub lightmap: LI32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct MipHeader {
    pub texture_len: LU32,
    pub offset: LU32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct MipTexture {
    pub name: [LI8; 16],
    pub width: LU32,
    pub height: LU32,
    pub offsets: [LU32; 4],
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Node {
    pub plane_id: LI32,
    pub front_id: LI16,
    pub back_id: LI16,
    pub bounds: BBoxShort,
    pub face_id: LU16,
    pub face_len: LU16,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Sounds {
    pub water: LU8,
    pub sky: LU8,
    pub slime: LU8,
    pub lava: LU8,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Leaf {
    pub leaf_type: LI32,
    pub vis_index: LI32,
    pub bounds: BBoxShort,
    pub face_index_id: LU16,
    pub face_index_len: LU16,
    pub sounds: Sounds,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Plane {
    pub normal: Scalar3,
    pub dist: Scalar,
    pub plane_type: LI32,
}
