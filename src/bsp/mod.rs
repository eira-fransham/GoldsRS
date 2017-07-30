use std::mem;
use std::slice;
use std::marker::PhantomData;
use std::borrow::Cow;

use ioendian::IntoNativeEndian;

use sys::bsp as sys;

pub use sys::bsp::{BoundingBox, Vec3, Quake1Lump, UnifiesWith};

pub mod mapversions;
pub mod quake1;

use self::quake1::*;

pub use self::mapversions::MapVersion;

pub trait FromBsp<'a, Src, V> {
    fn from_bsp(bsp: &'a Bsp<'a, V>, from: &'a Src) -> Self;
}

pub struct Bsp<'a, V>(Cow<'a, [u8]>, PhantomData<V>);

impl<'a, V> ::std::fmt::Debug for Bsp<'a, V> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        let length = self.0.as_ref().len();

        write!(f, "Bsp {{ ... {} bytes }}", length)
    }
}

pub struct ValueIter<'a, V: 'a, Src, Dst> {
    bsp: &'a Bsp<'a, V>,
    start: *const Src,
    end: *const Src,
    output: PhantomData<Dst>,
}

impl<'a, V, Src, Dst> ValueIter<'a, V, Src, Dst> {
    unsafe fn new(bsp: &'a Bsp<'a, V>, slice: &[Src]) -> Self {
        ValueIter {
            bsp: bsp,
            start: slice.as_ptr(),
            end: slice.as_ptr().offset(slice.len() as _),
            output: PhantomData,
        }
    }
}

impl<'a, V, Src: 'a, Dst: FromBsp<'a, Src, V>> Iterator for ValueIter<'a, V, Src, Dst> {
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

#[derive(Debug, Clone)]
pub enum Error {
    VersionMismatch(u32),
    HeaderCorrupted,
    EntryCorrupted(&'static str),
}

impl<'a, V: MapVersion<Lump = sys::Quake1Lump> + 'a> Bsp<'a, V> {
    pub fn into_static(self) -> Bsp<'static, V> {
        Bsp(Cow::Owned(self.0.into_owned()), PhantomData)
    }

    pub unsafe fn new_unchecked<T: Into<Cow<'a, [u8]>>>(buffer: T) -> Self {
        Bsp(buffer.into(), PhantomData)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn new<T: Into<Cow<'a, [u8]>>>(buffer: T) -> Result<Self, Error> {
        let unchecked = unsafe { Self::new_unchecked(buffer) };
        if unchecked.len() < mem::size_of::<sys::Header<V::Magic, V::Lump>>() {
            return Err(Error::HeaderCorrupted);
        }

        {
            let h = unchecked.header();

            if !V::accepts_version(h.version.native()) {
                return Err(Error::VersionMismatch(h.version.native()));
            }

            for &(ref entry, ref name) in
                &[
                    (&h.lumps.entities.clone().transmute::<sys::Entry>(), "entities"),
                    (&h.lumps.planes.clone().transmute(), "planes"),
                    (&h.lumps.miptex.clone().transmute(), "miptex"),
                    (&h.lumps.vertices.clone().transmute(), "vertices"),
                    (&h.lumps.vislist.clone().transmute(), "vislist"),
                    (&h.lumps.nodes.clone().transmute(), "nodes"),
                    (&h.lumps.texinfo.clone().transmute(), "texinfo"),
                    (&h.lumps.faces.clone().transmute(), "faces"),
                    (&h.lumps.lightmaps.clone().transmute(), "lightmaps"),
                    (&h.lumps.clipnodes.clone().transmute(), "clipnodes"),
                    (&h.lumps.leaves.clone().transmute(), "leaves"),
                    (&h.lumps.lfaces.clone().transmute(), "lfaces"),
                    (&h.lumps.edges.clone().transmute(), "edges"),
                    (&h.lumps.ledges.clone().transmute(), "ledges"),
                    (&h.lumps.models.clone().transmute(), "models"),
                ]
            {
                if !entry
                    .offset
                    .native()
                    .checked_add(entry.len.native())
                    .map(|end| (end as usize) <= unchecked.len())
                    .unwrap_or(false)
                {
                    return Err(Error::EntryCorrupted(name));
                }
            }
        }

        Ok(unchecked)
    }

    unsafe fn slice_from_header<'b, T, U: UnifiesWith<T>>(
        &'b self,
        header: &'b sys::Entry<U>,
    ) -> &'b [T] {
        self.slice_ref(
            header.offset.native() as _,
            (header.len.native() as usize) / mem::size_of::<T>(),
        )
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

    fn header(&self) -> &sys::Header<V::Magic, V::Lump> {
        unsafe { self.value_ref(0) }
    }

    fn faces(&self) -> &[sys::Face] {
        unsafe { self.slice_from_header(&self.header().lumps.faces) }
    }

    fn edges(&self) -> &[sys::Edge] {
        unsafe { self.slice_from_header(&self.header().lumps.edges) }
    }

    fn vertices(&self) -> &[sys::Scalar3] {
        unsafe { self.slice_from_header(&self.header().lumps.vertices) }
    }

    fn planes(&self) -> &[sys::Plane] {
        unsafe { self.slice_from_header(&self.header().lumps.planes) }
    }

    fn models(&self) -> &[sys::Model] {
        unsafe { self.slice_from_header(&self.header().lumps.models) }
    }

    fn branches(&self) -> &[sys::Node] {
        unsafe { self.slice_from_header(&self.header().lumps.nodes) }
    }

    fn leaves(&self) -> &[sys::Leaf] {
        unsafe { self.slice_from_header(&self.header().lumps.leaves) }
    }

    fn vislist(&self) -> &[u8] {
        unsafe { self.slice_from_header(&self.header().lumps.vislist) }
    }

    fn face_indices(&self) -> &[FaceRef] {
        unsafe { self.slice_from_header(&self.header().lumps.lfaces) }
    }

    fn edge_indices(&self) -> &[EdgeRef] {
        unsafe { self.slice_from_header(&self.header().lumps.ledges) }
    }

    pub fn leaf(&self, index: usize) -> Option<Leaf<V>> {
        let leaf: Leaf<V> = FromBsp::from_bsp(self, &self.leaves()[index]);
        if leaf.is_invalid() { None } else { Some(leaf) }
    }

    pub fn branch(&self, index: usize) -> Branch<V> {
        FromBsp::from_bsp(self, &self.branches()[index])
    }

    pub fn plane(&self, index: usize) -> Plane {
        FromBsp::from_bsp(self, &self.planes()[index])
    }

    fn node(&self, id_with_flag: i32) -> Option<Node<V>> {
        let is_leaf = id_with_flag < 0;
        let id: u16 = if is_leaf {
            (-id_with_flag - 1) as _
        } else {
            id_with_flag as _
        };

        if is_leaf {
            self.leaf(id as _).map(Node::Leaf)
        } else {
            Some(Node::Branch(self.branch(id as _)))
        }
    }

    pub fn map_model(&self) -> Model<V> {
        Model::from_bsp(self, &self.models()[0])
    }

    pub fn root(&self) -> Option<Node<V>> {
        self.map_model().root()
    }
}
