mod editing;
mod history;
mod indent;
mod io;
mod kind;
mod model;

#[cfg(test)]
mod tests;

pub use indent::IndentStyle;
pub use io::LineEnding;
pub use kind::BufferKind;
pub use model::{Buffer, BufferEdit, Position};
