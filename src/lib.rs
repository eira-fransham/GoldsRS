#![cfg_attr(feature = "nightly", feature(nonzero))]

#[cfg(feature = "nightly")]
extern crate core;

extern crate ioendian;

mod sys;
pub mod bsp;

#[cfg(test)]
mod tests {
    use bsp::*;
    use bsp::quake1::*;

    #[allow(dead_code)]
    fn print_node<T: MapVersion<Lump = Quake1Lump>>(node: &Node<T>) {
        use std::mem;

        fn print_node_inner<S: MapVersion<Lump = Quake1Lump>>(o_is_front: Option<bool>, prefix: String, node: Option<&Node<S>>) {
            let init = o_is_front
                .map(|is_front| if is_front { "├─" } else { "└─" })
                .unwrap_or("");

            match node {
                Some(&Node::Branch(ref inner)) => {
                    let bounds: [[u16; 3]; 2] = unsafe { mem::transmute(inner.bounds()) };

                    println!("{}{}{:?}", prefix, init, bounds);

                    let new_prefix = if o_is_front.unwrap_or(false) {
                        prefix + "│ "
                    } else {
                        prefix + "  "
                    };

                    print_node_inner(Some(true), new_prefix.clone(), inner.back().as_ref());
                    print_node_inner(Some(false), new_prefix, inner.front().as_ref());
                }
                Some(&Node::Leaf(ref inner)) => {
                    let bounds: [[u16; 3]; 2] = unsafe { mem::transmute(inner.bounds()) };
                    println!("{}{}{:?} - {:?}", prefix, init, bounds, inner.leaf_type());
                }
                None => {
                    println!("{}{}(none)", prefix, init);
                }
            }
        }

        print_node_inner(None, Default::default(), Some(&node));
    }

    #[test]
    fn quake_dm1() {
        use bsp::mapversions::Quake1;

        static DM1: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/DM1.BSP"));

        let bsp: Bsp<Quake1> = Bsp::new(DM1).unwrap();

        let map = bsp.map_model();

        let leaf = map.root()
            .unwrap()
            .branch()
            .unwrap()
            .traverse(&Vec3 {
                x: 20184,
                y: -12445,
                z: -21673,
            })
            .unwrap();

        let bounds_as_array: [[u16; 3]; 2] = unsafe { ::std::mem::transmute(leaf.bounds()) };

        assert_eq!(bounds_as_array, [[544, 536, 0], [672, 800, 218]]);
    }
}
