#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

extern crate rustc_serialize;


#[cfg(test)]
pub mod addons;

#[cfg(test)]
pub mod tables;

// FIXME: Can this go into "tests/" as is standard? Previous usage of build.rs probably made that
// difficult.


domain! { pub DOCTEST }
table! {
    /// Can we document the table?
    pub [DOCTEST/documentation] {
        /// Can we document this column?
        documented_column: [bool; BoolCol],
        undocumented_column: [bool; BoolCol],
    }
}