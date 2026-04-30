use llnzy::workspace_store;

use crate::App;

impl App {
    pub(crate) fn restore_last_session(&mut self) {
        workspace_store::clear_last_session();
    }
}
