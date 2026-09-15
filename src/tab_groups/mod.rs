mod model;

pub use model::{PartitionAxis, TabGroup, TabGroupId, TabGroupState, TabId};

/// Fixed capacity of a joined workspace group.
pub const MAX_JOINED_TABS: usize = 4;
