use core::nonzero::NonZero;

use std::slice;
use std::marker::PhantomData;
use std::borrow::Cow;

use ioendian::{IntoNativeEndian, Little};

use sys::bsp as sys;

pub use sys::bsp::{BoundingBox, Vec3, Scalar3, Short3};

pub trait FromBsp<'a, Src> {
    fn from_bsp(bsp: &'a Bsp<'a>, from: &'a Src) -> Self;
}

pub struct Bsp<'a>(Cow<'a, [u8]>);

impl<'a> ::std::fmt::Debug for Bsp<'a> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        let length = self.0.as_ref().len();

        write!(f, "Bsp {{ ... {} bytes }}", length)
    }
}

pub struct Branch<'a>(&'a sys::Node, &'a Bsp<'a>);
pub struct Leaf<'a>(&'a sys::Leaf, &'a Bsp<'a>);

pub enum Node<'a> {
    Branch(Branch<'a>),
    Leaf(Leaf<'a>),
}

impl<'a> Node<'a> {
    pub fn branch(self) -> Option<Branch<'a>> {
        match self {
            Node::Branch(inner) => Some(inner),
            Node::Leaf(_) => None,
        }
    }

    pub fn leaf(self) -> Option<Leaf<'a>> {
        match self {
            Node::Branch(_) => None,
            Node::Leaf(inner) => Some(inner),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EdgeRef(Little<u32>);
#[derive(Copy, Clone, Debug)]
pub struct FaceRef(Little<u16>);

pub enum Side {
    Back,
    Front,
}

pub enum PlaneType {
    AxialX,
    AxialY,
    AxialZ,
    NonAxialX,
    NonAxialY,
    NonAxialZ,
}

pub type Bounds = BoundingBox<Vec3<i16>>;

pub struct Plane {
    pub normal: Vec3<f32>,
    pub dist: f32,
    pub plane_type: PlaneType,
}

impl<'a, Src: Clone + Into<Dst>, Dst> FromBsp<'a, Src> for Dst {
    fn from_bsp(_: &'a Bsp<'a>, from: &'a Src) -> Dst {
        from.clone().into()
    }
}

impl From<sys::Plane> for Plane {
    fn from(other: sys::Plane) -> Self {
        use self::PlaneType::*;

        Plane {
            normal: Vec3 {
                x: other.normal.x.native(),
                y: other.normal.y.native(),
                z: other.normal.z.native(),
            },
            dist: other.dist.native(),
            plane_type: match other.plane_type.native() {
                0 => AxialX,
                1 => AxialY,
                2 => AxialZ,
                3 => NonAxialX,
                4 => NonAxialY,
                5 => NonAxialZ,
                _ => panic!("Invalid plane type: {}", other.plane_type),
            },
        }
    }
}

// TODO: Make this lazily pull start and end from the BSP
pub struct Edge<'a> {
    start: &'a Scalar3,
    end: &'a Scalar3,
}

impl<'a> Edge<'a> {
    pub fn start(&self) -> Vec3<f32> {
        self.start.native()
    }

    pub fn end(&self) -> Vec3<f32> {
        self.end.native()
    }
}

impl<'a> FromBsp<'a, sys::Leaf> for Leaf<'a> {
    fn from_bsp(bsp: &'a Bsp, leaf: &'a sys::Leaf) -> Self {
        Leaf(leaf, bsp)
    }
}

impl<'a> FromBsp<'a, sys::Node> for Branch<'a> {
    fn from_bsp(bsp: &'a Bsp, leaf: &'a sys::Node) -> Self {
        Branch(leaf, bsp)
    }
}

impl<'a> FromBsp<'a, sys::Edge> for Edge<'a> {
    fn from_bsp(bsp: &'a Bsp, edge: &'a sys::Edge) -> Edge<'a> {
        let verts = bsp.vertices();

        Edge {
            start: &verts[edge.start.native() as usize],
            end: &verts[edge.end.native() as usize],
        }
    }
}

impl<'a> FromBsp<'a, FaceRef> for Face<'a> {
    fn from_bsp(bsp: &'a Bsp, face_ref: &'a FaceRef) -> Face<'a> {
        Self::from_bsp(bsp, &bsp.faces()[face_ref.0.native() as usize])
    }
}

impl<'a> FromBsp<'a, EdgeRef> for Edge<'a> {
    fn from_bsp(bsp: &'a Bsp, edge_ref: &'a EdgeRef) -> Edge<'a> {
        Self::from_bsp(bsp, &bsp.edges()[edge_ref.0.native() as usize])
    }
}

pub struct Face<'a>(&'a sys::Face, &'a Bsp<'a>);

impl<'a> FromBsp<'a, sys::Face> for Face<'a> {
    fn from_bsp(bsp: &'a Bsp, from: &'a sys::Face) -> Self {
        Face(from, bsp)
    }
}

#[derive(Debug)]
pub struct Model<'a>(&'a sys::Model, &'a Bsp<'a>);

impl<'a> FromBsp<'a, sys::Model> for Model<'a> {
    fn from_bsp(bsp: &'a Bsp, from: &'a sys::Model) -> Self {
        Model(from, bsp)
    }
}

impl<'a> Model<'a> {
    pub fn root(&self) -> Node<'a> {
        let id_with_flag = self.0.hulls[0].native();
        let is_leaf = id_with_flag < 0;
        let id: u16 = if is_leaf {
            (-id_with_flag) as _
        } else {
            id_with_flag as _
        };

        if is_leaf {
            Node::Leaf(self.1.leaf(id as _))
        } else {
            Node::Branch(self.1.node(id as _))
        }
    }
}

pub struct ValueIter<'a, Src, Dst: FromBsp<'a, Src>> {
    bsp: &'a Bsp<'a>,
    start: *const Src,
    end: *const Src,
    output: PhantomData<Dst>,
}

impl<'a, Src, Dst: FromBsp<'a, Src>> ValueIter<'a, Src, Dst> {
    unsafe fn new(bsp: &'a Bsp<'a>, slice: &[Src]) -> Self {
        ValueIter {
            bsp: bsp,
            start: slice.as_ptr(),
            end: slice.as_ptr().offset(slice.len() as _),
            output: PhantomData,
        }
    }
}

impl<'a, Src: 'a, Dst: FromBsp<'a, Src>> Iterator for ValueIter<'a, Src, Dst> {
    type Item = Dst;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            return None;
        }

        let next = self.start;
        self.start = unsafe { self.start.offset(1) };

        Some(Dst::from_bsp(self.bsp, unsafe { &*next }))
    }
}

impl<'a> Bsp<'a> {
    pub fn into_static(self) -> Bsp<'static> {
        Bsp(Cow::Owned(self.0.into_owned()))
    }

    pub unsafe fn new<T: Into<Cow<'a, [u8]>>>(buffer: T) -> Self {
        let out = Bsp(buffer.into());
        debug_assert_eq!(out.header().version.native(), 0x1d);
        out
    }

    unsafe fn slice_from_header<'b, T>(&'b self, header: &'b sys::Entry) -> &'b [T] {
        self.slice_ref(header.offset.native() as _, header.len.native() as _)
    }

    #[inline(always)]
    unsafe fn slice_ref<'b, T>(&'b self, offset: usize, count: usize) -> &'b [T] {
        debug_assert!(
            offset
                .checked_add(count)
                .map(|e| e <= self.0.len())
                .unwrap_or(false)
        );

        slice::from_raw_parts(self.0.as_ptr().offset(offset as _) as _, count)
    }

    #[inline(always)]
    unsafe fn value_ref<'b, T>(&'b self, offset: usize) -> &'b T {
        &*(self.0.as_ptr().offset(offset as _) as *const T)
    }

    fn header(&self) -> &sys::Header {
        unsafe { self.value_ref(0) }
    }

    fn faces(&self) -> &[sys::Face] {
        unsafe { self.slice_from_header(&self.header().faces) }
    }

    fn edges(&self) -> &[sys::Edge] {
        unsafe { self.slice_from_header(&self.header().edges) }
    }

    fn vertices(&self) -> &[sys::Scalar3] {
        unsafe { self.slice_from_header(&self.header().vertices) }
    }

    fn planes(&self) -> &[sys::Plane] {
        unsafe { self.slice_from_header(&self.header().vertices) }
    }

    fn models(&self) -> &[sys::Model] {
        unsafe { self.slice_from_header(&self.header().models) }
    }

    fn nodes(&self) -> &[sys::Node] {
        unsafe { self.slice_from_header(&self.header().nodes) }
    }

    fn leaves(&self) -> &[sys::Leaf] {
        unsafe { self.slice_from_header(&self.header().leaves) }
    }

    fn vislist(&self) -> &[u8] {
        unsafe { self.slice_from_header(&self.header().vislist) }
    }

    fn face_indices(&self) -> &[FaceRef] {
        unsafe { self.slice_from_header(&self.header().lface) }
    }

    pub fn leaf(&self, index: u16) -> Leaf {
        FromBsp::from_bsp(self, &self.leaves()[index as usize])
    }

    pub fn node(&self, index: u16) -> Branch {
        FromBsp::from_bsp(self, &self.nodes()[index as usize])
    }

    pub fn map_model(&self) -> Model {
        Model::from_bsp(self, &self.models()[0])
    }

    pub fn root(&self) -> Node {
        self.map_model().root()
    }
}

impl<'a> Branch<'a> {
    pub fn plane(&'a self) -> Plane {
        self.1.planes()[self.0.plane_id.native() as usize]
            .clone()
            .into()
    }

    pub fn front(&self) -> Node<'a> {
        let id_with_flag = self.0.front_id.native();
        let is_leaf = id_with_flag < 0;
        let id: u16 = if is_leaf {
            (-id_with_flag) as _
        } else {
            id_with_flag as _
        };

        if is_leaf {
            Node::Leaf(self.1.leaf(id))
        } else {
            Node::Branch(self.1.node(id))
        }
    }

    pub fn back(&self) -> Node<'a> {
        let id_with_flag = self.0.back_id.native();
        let is_leaf = id_with_flag < 0;
        let id: u16 = if is_leaf {
            (-id_with_flag) as _
        } else {
            id_with_flag as _
        };

        if is_leaf {
            Node::Leaf(self.1.leaf(id))
        } else {
            Node::Branch(self.1.node(id))
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
}

#[repr(i32)]
pub enum LeafType {
    Ordinary = -1,
    Invalid = -2,
    Water = -3,
    Slime = -4,
    Lava = -5,
    Sky = -6,
}

pub struct VisibilityIterator<'a> {
    bsp: &'a Bsp<'a>,
    vis_list: &'a [u8],
    num_leaves: usize,
    index: usize,
    other_index: usize,
    bit: Option<NonZero<u8>>,
}

impl<'a> Iterator for VisibilityIterator<'a> {
    type Item = Leaf<'a>;

    // TODO: Prevent this from being recursive
    fn next(&mut self) -> Option<Self::Item> {
        if self.other_index > self.num_leaves {
            None
        } else if let Some(bit) = self.bit {
            if bit.get() >= 8 {
                self.bit = None;
                self.index += 1;

                self.next()
            } else {
                let other_index = self.other_index;
                self.other_index += 1;

                let mask = 2 << bit.get();
                self.bit = NonZero::new(bit.get() + 1);
                if self.vis_list[self.index] & mask != 0 {
                    Some(self.bsp.leaf(other_index as _))
                } else {
                    self.next()
                }
            }
        } else if self.vis_list[self.index] == 0 {
            self.other_index += 8 * self.vis_list[self.index + 1] as usize;
            self.index += 2;

            self.next()
        } else {
            self.bit = NonZero::new(1u8);

            self.next()
        }
    }
}

impl<'a> Leaf<'a> {
    pub fn leaf_type(&self) -> LeafType {
        use std::mem;

        let lty = self.0.leaf_type.native();
        assert!(lty >= LeafType::Ordinary as i32 && lty <= LeafType::Sky as i32);
        unsafe { mem::transmute(lty) }
    }

    // TODO: Do this without allocating
    pub fn visible_leaves(&self) -> VisibilityIterator {
        let num_leaves = self.1.leaves().len();
        let vis_list = self.1.vislist();

        let my_index = self.0.vis_index.native() as usize;
        let other_index = 1;

        VisibilityIterator {
            bsp: self.1,
            bit: None,
            num_leaves: num_leaves,
            vis_list: vis_list,
            index: my_index,
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

    pub fn faces(&self) -> ValueIter<FaceRef, Face> {
        let start = self.0.face_index_id.native() as usize;
        let end = start + self.0.face_index_len.native() as usize;
        unsafe { ValueIter::new(self.1, &self.1.face_indices()[start..end]) }
    }
}
