use vec1::{vec1, Vec1};

use indenty::{tree, RoseTree};

fn main() {
    let test_tree = tree![0 =>
          tree![1],
          tree![2],
          tree![3 => tree![4]]
    ];
    let mut v = vec![];
    test_tree.to_doc(false).render(640, &mut v).unwrap();
    let result = String::from_utf8(v).unwrap();
    println!("{}", result);
}
