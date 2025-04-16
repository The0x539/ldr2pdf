use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use notify::{EventHandler, RecursiveMode, Watcher, event::*};

pub fn file_touched(path: &Path) -> impl FnMut() -> bool + use<> {
    let flag = Flag::default();

    let mut watcher = notify::recommended_watcher(flag.clone()).unwrap();
    watcher.watch(path, RecursiveMode::NonRecursive).unwrap();

    move || {
        _ = &watcher; // Move the watcher into the closure to keep it alive
        flag.take()
    }
}

#[derive(Default, Clone)]
struct Flag(Arc<AtomicBool>);

impl Flag {
    fn set(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    fn take(&self) -> bool {
        self.0
            .compare_exchange_weak(true, false, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    }
}

impl EventHandler for Flag {
    fn handle_event(&mut self, event: notify::Result<Event>) {
        let Ok(event) = event else { return };

        if event.kind.is_remove()
            || event.kind == EventKind::Modify(ModifyKind::Name(RenameMode::From))
        {
            return;
        }

        self.set();
    }
}
