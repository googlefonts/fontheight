use pyo3::{pymodule, Bound, PyResult};
use pyo3::prelude::*;

#[pyfunction]
fn hello_world() {
    println!("Hello world");
}

#[pymodule]
fn fontheight(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(hello_world, module)?)?;
    Ok(())
}
