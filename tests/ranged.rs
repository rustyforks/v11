#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

use v11::*;
use v11::tracking::prelude::*;
use v11::tables::RowRange;



// FIXME: Test sorting.

// FIXME: Can this go into "tests/" as is standard? Previous usage of build.rs probably made that
// difficult.

domain! { pub TEST }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Blah;

table! {
    /// FIXME: Does this document the table?
    #[kind = "consistent"]
    [TEST/elements] {
        bits: [bool; BoolCol],
        bytes: [u8; VecCol<u8>],
        blah: [Blah; VecCol<Blah>],
    }
}
table! {
    #[kind = "consistent"]
    #[row_derive(Clone)]
    [TEST/arrays] {
        #[foreign]
        #[index]
        range_start: [elements::RowId; VecCol<elements::RowId>],
        // FIXME: foreign gives us a `usize` on the events, which isn't convertible to a RowRange.
        // There's a less trivial way to work around this that we're too lazy to try here.
        range: [RowRange<elements::RowId>; VecCol<RowRange<elements::RowId>>],
    }
}

impl Tracker for arrays::track_range_start_events {
    type Foreign = elements::Row;

    fn sort(&self) -> bool { false }

    fn handle(
        &self,
        universe: &Universe,
        event: Event,
        selected: SelectRows<Self::Foreign>,
        function: &dyn v11::event::Function,
    ) {
        let mut rows = arrays::read(universe).select_range_start(selected);
        let gt = arrays::get_generic_table(universe);
        if function.needs_sort(gt) {
            rows.sort();
        }
        let rows = rows.as_slice();
        let rows = rows.as_any();
        function.handle(universe, gt, event, rows);
    }
}

fn verse() -> Universe {
    TEST.register();
    elements::register();
    arrays::register();
    Universe::new(&[TEST])
}

#[test]
fn ranged() {
    println!();
    let universe = &verse();
    let mut e = elements::write(universe);
    let mut a = arrays::write(universe);
    {

        for n in 0..3 {
            let start = e.push(elements::Row {
                bits: true,
                bytes: 42,
                blah: Blah,
            });
            let mut end = start;
            for _ in 0..n {
                end = e.push(elements::Row {
                    bits: false,
                    bytes: 24,
                    blah: Blah,
                });
            }
            a.push(arrays::Row {
                range_start: start,
                range: RowRange {
                    start: start,
                    end: end.next(),
                },
            });
        }
    }

    for row in a.iter() {
        println!("{:?}: {:?}", row, a.range[row]);
        for erow in e.range(a.range[row]) {
            println!("\t{}", e.bits[erow]);
        }
        // RowRange is copy, so this does the full iteration twice.
        println!("{:?}: {:?}", row, a.range[row]);
        for erow in e.range(a.range[row]) {
            println!("\t{}", e.bits[erow]);
        }
    }
    a.flush(universe, event::CREATE);
    e.flush(universe, event::CREATE);
}
