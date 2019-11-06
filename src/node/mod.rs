pub mod tonemap;

#[derive(Default)]
pub struct Aux {
    pub frames: usize,
    pub hw_alignment: u64,
    pub tonemapper_args: tonemap::TonemapperArgs
}
