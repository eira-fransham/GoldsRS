#![feature(nonzero)]

extern crate core;
extern crate ioendian;
extern crate memmap;

mod sys;
pub mod bsp;

#[cfg(test)]
mod tests {
    use bsp::*;

    fn print_node(node: &Node) {
        use std::mem;

        fn print_node_inner(o_is_front: Option<bool>, prefix: String, node: &Node) {
            match *node {
                Node::Branch(ref inner) => {
                    let bounds: [[u16; 3]; 2] = unsafe { mem::transmute(inner.bounds()) };
                    let init = o_is_front
                        .map(|is_front| if is_front { "├─" } else { "└─" })
                        .unwrap_or("");

                    println!("{}{}{:?}", prefix, init, bounds);

                    let new_prefix = if o_is_front.unwrap_or(false) {
                        prefix + "│ "
                    } else {
                        prefix + "  "
                    };

                    print_node_inner(Some(true), new_prefix.clone(), &inner.back());
                    print_node_inner(Some(false), new_prefix, &inner.front());
                }
                Node::Leaf(ref inner) => {
                    let bounds: [[u16; 3]; 2] = unsafe { mem::transmute(inner.bounds()) };
                    let init = o_is_front
                        .map(|is_front| if is_front { "├──" } else { "└──" })
                        .unwrap_or("");

                    println!("{}{}{:?}", prefix, init, bounds);
                }
            }
        }

        print_node_inner(None, Default::default(), &node);
    }

    #[test]
    fn simple_dm5() {
        use memmap::{Mmap, Protection};

        let simple_dm5: Mmap = Mmap::open_path(
            concat!(env!("CARGO_MANIFEST_DIR"), "/assets/DM1.BSP"),
            Protection::Read,
        ).expect("Opening BSP failed");

        let bsp: Bsp = unsafe { Bsp::new(simple_dm5.as_slice()) };

        let map = bsp.map_model();
        let leaf = map.root()
            .branch()
            .unwrap()
            .front()
            .branch()
            .unwrap()
            .back()
            .branch()
            .unwrap()
            .front()
            .branch()
            .unwrap()
            .front()
            .leaf()
            .unwrap();

        let bounds_as_array: [[u16; 3]; 2] = unsafe { ::std::mem::transmute(leaf.bounds()) };

        assert_eq!(bounds_as_array, [[632, 928, 248], [864, 1360, 306]]);
    }
}
