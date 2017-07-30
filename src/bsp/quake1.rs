use std::borrow::Cow;
use std::marker::PhantomData;

use bsp::{Bsp, ValueIter, FromBsp, BoundingBox, Vec3};
use bsp::mapversions::MapVersion;

use ioendian::{Little, IntoNativeEndian};

use sys::bsp as sys;
use sys::bsp::Scalar3;

#[cfg(feature = "nightly")]
pub struct VisibilityIterator<'a, V: 'a> {
    bsp: &'a Bsp<'a, V>,
    vis_list: &'a [u8],
    num_leaves: usize,
    index: u32,
    all: bool,
    other_index: usize,
    bit: Option<core::nonzero::NonZero<u8>>,
}

#[cfg(not(feature = "nightly"))]
pub struct VisibilityIterator<'a, V: 'a> {
    bsp: &'a Bsp<'a, V>,
    vis_list: &'a [u8],
    num_leaves: usize,
    index: u32,
    all: bool,
    other_index: usize,
    bit: Option<u8>,
}

impl<'a, V: MapVersion<Lump = sys::Quake1Lump>> Iterator for VisibilityIterator<'a, V> {
    type Item = Leaf<'a, V>;

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(not(feature = "nightly"))]
        fn nonzero(b: u8) -> Option<u8> {
            debug_assert!(b != 0);
            Some(b)
        }

        #[cfg(feature = "nightly")]
        fn nonzero(b: u8) -> Option<core::nonzero::NonZero<u8>> {
            core::nonzero::NonZero::new(b)
        }

        #[cfg(not(feature = "nightly"))]
        fn get(b: u8) -> u8 {
            b
        }

        #[cfg(feature = "nightly")]
        fn get(b: core::nonzero::NonZero<u8>) -> u8 {
            b.get()
        }

        loop {
            if self.all {
                let out = self.bsp.leaf(self.other_index as _);
                self.other_index += 1;

                if let Some(out) = out {
                    break Some(out);
                }
            } else if self.other_index > self.num_leaves {
                break None;
            } else if let Some(bit) = self.bit {
                if get(bit) >= 8 {
                    self.bit = None;
                    self.index += 1;
                } else {
                    let other_index = self.other_index;
                    self.other_index += 1;

                    let mask = 2 << get(bit);
                    self.bit = nonzero(get(bit) + 1);

                    if self.vis_list[self.index as usize] & mask != 0 {
                        break Some(self.bsp.leaf(other_index as _).expect(
                            "Leaf can see invalid leaf",
                        ));
                    }
                }
            } else if self.vis_list[self.index as usize] == 0 {
                self.other_index += 8 * self.vis_list[self.index as usize + 1] as usize;
                self.index += 2;
            } else {
                self.bit = nonzero(1u8);
            }
        }
    }
}

pub struct Branch<'a, V: 'a>(&'a sys::Node, &'a Bsp<'a, V>);
pub struct Leaf<'a, V: 'a>(&'a sys::Leaf, &'a Bsp<'a, V>);

#[repr(i8)]
#[derive(Debug)]
pub enum LeafType {
    Ordinary = -1,
    Water = -3,
    Slime = -4,
    Lava = -5,
    Sky = -6,
}

impl<'a, V: 'a> Clone for Branch<'a, V> {
    fn clone(&self) -> Self {
        Branch(self.0, self.1)
    }
}

impl<'a, V: MapVersion<Lump = sys::Quake1Lump>> Branch<'a, V> {
    pub fn plane(&self) -> Plane {
        self.1.plane(self.0.plane_id.native() as _)
    }

    pub fn front(&self) -> Option<Node<'a, V>> {
        self.1.node(self.0.front_id.native() as _)
    }

    pub fn back(&self) -> Option<Node<'a, V>> {
        self.1.node(self.0.back_id.native() as _)
    }

    pub fn bounds(&self) -> Bounds {
        let bounds = &self.0.bounds;
        let aa = &bounds.aa;
        let bb = &bounds.bb;

        Bounds {
            aa: Vec3 {
                x: aa.x.native(),
                y: aa.y.native(),
                z: aa.z.native(),
            },
            bb: Vec3 {
                x: bb.x.native(),
                y: bb.y.native(),
                z: bb.z.native(),
            },
        }
    }

    pub fn traverse(&self, position: &Vec3<i16>) -> Option<Leaf<'a, V>> {
        fn dot(a: &Vec3<f32>, b: &Vec3<f32>) -> f32 {
            a.x * b.x + a.y * b.y + a.z * b.z
        }

        let mut node = Cow::Borrowed(self);
        let fpos = Vec3 {
            x: position.x as _,
            y: position.y as _,
            z: position.z as _,
        };

        loop {
            let plane = node.plane();

            let o_out = if dot(&plane.normal, &fpos) - plane.distance >= 0. {
                node.front()
            } else {
                node.back()
            };

            match o_out {
                Some(Node::Branch(b)) => {
                    node = Cow::Owned(b);
                }
                Some(Node::Leaf(l)) => {
                    break Some(l);
                }
                None => break None,
            }
        }
    }
}

impl<'a, V: MapVersion<Lump = sys::Quake1Lump>> Leaf<'a, V> {
    pub fn leaf_type(&self) -> LeafType {
        use std::mem;

        let lty = self.0.leaf_type.native() as i8;
        assert!(lty >= LeafType::Sky as i8 && lty <= LeafType::Ordinary as i8);
        unsafe { mem::transmute(lty) }
    }

    pub fn is_invalid(&self) -> bool {
        const INVALID: i32 = -2;

        self.0.leaf_type.native() == INVALID
    }

    pub fn visible_leaves(&self) -> VisibilityIterator<V> {
        let num_leaves = self.1.leaves().len();
        let vis_list = self.1.vislist();

        let my_index = self.0.vis_index.native();
        let other_index = 1;

        VisibilityIterator {
            bsp: self.1,
            bit: None,
            num_leaves: num_leaves,
            vis_list: vis_list,
            all: my_index < 0,
            index: my_index as _,
            other_index: other_index,
        }
    }

    pub fn bounds(&self) -> Bounds {
        let bounds = &self.0.bounds;
        let aa = &bounds.aa;
        let bb = &bounds.bb;

        Bounds {
            aa: Vec3 {
                x: aa.x.native(),
                y: aa.y.native(),
                z: aa.z.native(),
            },
            bb: Vec3 {
                x: bb.x.native(),
                y: bb.y.native(),
                z: bb.z.native(),
            },
        }
    }

    pub fn faces(&self) -> ValueIter<V, FaceRef, Face<V>> {
        let start = self.0.face_index_id.native() as usize;
        let end = start + self.0.face_index_len.native() as usize;
        unsafe { ValueIter::new(self.1, &self.1.face_indices()[start..end]) }
    }
}

pub enum Node<'a, V: 'a> {
    Branch(Branch<'a, V>),
    Leaf(Leaf<'a, V>),
}

impl<'a, V: 'a> Node<'a, V> {
    pub fn branch(self) -> Option<Branch<'a, V>> {
        match self {
            Node::Branch(inner) => Some(inner),
            Node::Leaf(_) => None,
        }
    }

    pub fn leaf(self) -> Option<Leaf<'a, V>> {
        match self {
            Node::Branch(_) => None,
            Node::Leaf(inner) => Some(inner),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EdgeRef(Little<i16>);
#[derive(Copy, Clone, Debug)]
pub struct FaceRef(Little<u16>);

impl sys::UnifiesWith<FaceRef> for Little<u16> {}
impl sys::UnifiesWith<EdgeRef> for Little<i16> {}

pub enum Side {
    Back,
    Front,
}

#[repr(u8)]
#[derive(Debug)]
pub enum PlaneType {
    AxialX = 0,
    AxialY = 1,
    AxialZ = 2,
    NonAxialX = 3,
    NonAxialY = 4,
    NonAxialZ = 5,
}

pub type Bounds = BoundingBox<Vec3<i16>>;

// TODO: Load this lazily from the BSP
#[derive(Debug)]
pub struct Plane {
    pub normal: Vec3<f32>,
    pub distance: f32,
    pub plane_type: PlaneType,
}

impl<'a, V, Src: Clone + Into<Dst>, Dst> FromBsp<'a, Src, V> for Dst {
    fn from_bsp(_: &'a Bsp<'a, V>, from: &'a Src) -> Self {
        from.clone().into()
    }
}

impl From<sys::Plane> for Plane {
    fn from(other: sys::Plane) -> Self {
        use std::mem;

        let plane_type = other.plane_type.native() as u8;
        assert!(plane_type <= PlaneType::NonAxialZ as u8 && plane_type >= PlaneType::AxialX as u8);

        Plane {
            normal: Vec3 {
                x: other.normal.x.native(),
                y: other.normal.y.native(),
                z: other.normal.z.native(),
            },
            distance: other.dist.native(),
            plane_type: unsafe { mem::transmute(plane_type) },
        }
    }
}

// TODO: Make this lazily pull start and end from the BSP
pub struct Edge<'a, V: 'a> {
    start: &'a Scalar3,
    end: &'a Scalar3,
    _phantom: PhantomData<V>,
}

impl<'a, V> Edge<'a, V> {
    pub fn start(&self) -> Vec3<f32> {
        self.start.native()
    }

    pub fn end(&self) -> Vec3<f32> {
        self.end.native()
    }
}

impl<'a, V> FromBsp<'a, sys::Leaf, V> for Leaf<'a, V> {
    fn from_bsp(bsp: &'a Bsp<'a, V>, from: &'a sys::Leaf) -> Self {
        Leaf(from, bsp)
    }
}

impl<'a, V> FromBsp<'a, sys::Node, V> for Branch<'a, V> {
    fn from_bsp(bsp: &'a Bsp<'a, V>, from: &'a sys::Node) -> Self {
        Branch(from, bsp)
    }
}

impl<'a, V: MapVersion<Lump = sys::Quake1Lump>> FromBsp<'a, sys::Edge, V> for Edge<'a, V> {
    fn from_bsp(bsp: &'a Bsp<'a, V>, from: &'a sys::Edge) -> Self {
        let verts = bsp.vertices();

        Edge {
            start: &verts[from.start.native() as usize],
            end: &verts[from.end.native() as usize],
            _phantom: PhantomData,
        }
    }
}

impl<'a, V: MapVersion<Lump = sys::Quake1Lump>> FromBsp<'a, FaceRef, V> for Face<'a, V> {
    fn from_bsp(bsp: &'a Bsp<'a, V>, from: &'a FaceRef) -> Self {
        Self::from_bsp(bsp, &bsp.faces()[from.0.native() as usize])
    }
}

impl<'a, V: MapVersion<Lump = sys::Quake1Lump>> FromBsp<'a, EdgeRef, V> for Edge<'a, V> {
    fn from_bsp(bsp: &'a Bsp<'a, V>, from: &'a EdgeRef) -> Self {
        Self::from_bsp(bsp, &bsp.edges()[from.0.native() as usize])
    }
}

pub struct Face<'a, V: 'a>(&'a sys::Face, &'a Bsp<'a, V>);

impl<'a, V: 'a> FromBsp<'a, sys::Face, V> for Face<'a, V> {
    fn from_bsp(bsp: &'a Bsp<'a, V>, from: &'a sys::Face) -> Self {
        Face(from, bsp)
    }
}

impl<'a, V: MapVersion<Lump = sys::Quake1Lump> + 'a> Face<'a, V> {
    pub fn plane(&self) -> Plane {
        let out_plane = self.1.plane(self.0.plane_id.native() as _);
        if self.0.side.native() == 0 {
            out_plane
        } else {
            Plane {
                normal: Vec3 {
                    x: -out_plane.normal.x,
                    y: -out_plane.normal.y,
                    z: -out_plane.normal.z,
                },
                distance: -out_plane.distance,
                ..out_plane
            }
        }
    }

    pub fn edges(&self) -> ValueIter<V, EdgeRef, Edge<V>> {
        let start = self.0.ledge_id.native() as usize;
        let end = start + self.0.ledge_len.native() as usize;
        unsafe { ValueIter::new(self.1, &self.1.edge_indices()[start..end]) }
    }

    #[cfg(feature = "nightly")]
    pub fn points(&self) -> impl Iterator<Item = Vec3<f32>> {
        // TODO: Some of these points are probably redundant
        self.edges().flat_map(|edge| [edge.start(), edge.end()])
    }
}

#[derive(Debug)]
pub struct Model<'a, V: 'a>(&'a sys::Model, &'a Bsp<'a, V>);

impl<'a, V: 'a> FromBsp<'a, sys::Model, V> for Model<'a, V> {
    fn from_bsp(bsp: &'a Bsp<'a, V>, from: &'a sys::Model) -> Self {
        Model(from, bsp)
    }
}

impl<'a, V: MapVersion<Lump = sys::Quake1Lump> + 'a> Model<'a, V> {
    pub fn root(&self) -> Option<Node<'a, V>> {
        self.1.node(self.0.hulls[0].native())
    }
}
