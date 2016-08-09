#![allow(dead_code)]


mod table_use {
    #[derive(Clone, Copy, PartialEq, PartialOrd, Debug, RustcEncodable, RustcDecodable)]
    pub enum CheeseKind {
        Swiss,
        Stinky,
        Brie,
    }
    impl Default for CheeseKind {
        fn default() -> Self { CheeseKind::Stinky }
    }
    impl ::Storable for CheeseKind { }
}
use self::table_use::*;

table! {
    [cheese],
    mass: usize,
    holes: u16,
    kind: CheeseKind,
}


#[test]
#[should_panic(expected = "Invalid name 123")]
fn bad_name() {
    let mut universe = ::Universe::new();
    cheese::with_name("123").register(&mut universe);
}

#[test]
fn small_table() {
    let mut universe = ::Universe::new();
    cheese::default().register(&mut universe);

    {
        let mut cheese = cheese::default().write(&universe);
        cheese.push(cheese::Row {
            mass: 1000usize,
            holes: 20,
            kind: CheeseKind::Stinky,
        });
    }
}

#[test]
fn large_table() {
    let mut universe = ::Universe::new();
    cheese::default().register(&mut universe);
    let mut cheese = cheese::default().write(&universe);
    for x in 10..1000 {
        cheese.push(cheese::Row {
            mass: x,
            holes: 2000,
            kind: CheeseKind::Swiss,
        });
    }
}

#[test]
fn walk_table() {
    let mut universe = ::Universe::new();
    cheese::default().register(&mut universe);
    {
        let mut cheese = cheese::write(&universe);
        for x in 10..1000 {
            cheese.push(cheese::Row {
                mass: x,
                holes: 2000,
                kind: CheeseKind::Swiss,
            });
        }
    }
    let cheese = cheese::read(&universe);
    for i in cheese.range() {
        println!("{:?}", cheese.row(i));
    }
}
