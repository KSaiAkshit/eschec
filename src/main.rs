use eschec::board::Positions;

fn main() {
    let size = std::mem::size_of::<Positions>();
    dbg!(size);
}
