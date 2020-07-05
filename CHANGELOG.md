# Version 0.0.0

Initial codebase.

# Version 0.0.1

Changes:
* Much better documentation, including a README.md file [#1]
* Added git pre-commit hooks [#1]
* Serialization is disabled by default behind a feature [#2]
* Don't infinite loop when a zero-length entry ends up in the vector [#3]

# Version 0.0.2

Changes:
* Added `#[derive(Clone)]` to make it cloneable [#7]

# Version 0.0.3

Changes:
* Added `.max_size()` function
* `.get()` and `.get_exact()` now return a reference to the `BumpyEntry` rather than a new `BumpyEntry` with a reference to the object `T`
* Add `.get_mut()` and `.get_exact_mut()` functions

# Version 0.0.4

Changes:
* Remove `iterate_over_empty` as an option
