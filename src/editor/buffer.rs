mod editing;
mod history;
mod indent;
mod io;
mod model;
mod recovery;

#[cfg(test)]
mod tests;

pub use indent::IndentStyle;
pub use io::LineEnding;
pub use model::{Buffer, BufferEdit, Position};
