use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

use multimap::MultiMap;
use notify::{Event, Watcher as _};

const DEBOUNCE_TIME: Duration = Duration::from_millis(5);

pub struct Watcher {
    rx: Receiver<notify::Result<Event>>,
    notifier: notify::RecommendedWatcher,
    dirs: MultiMap<PathBuf, PathBuf>,
    files: HashSet<PathBuf>,

    // debounce things
    last_time: Instant,
}

impl Watcher {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let notifier = notify::recommended_watcher(tx).expect("Failed to create file watcher");
        Self {
            rx,
            dirs: Default::default(),
            files: Default::default(),
            notifier,
            last_time: Instant::now(),
        }
    }
    pub fn add_file(&mut self, file_path: PathBuf) -> std::io::Result<()> {
        let file_path = file_path;
        let parent = file_path.parent().unwrap_or(Path::new("."));
        self.notifier
            .watch(parent, notify::RecursiveMode::NonRecursive)
            .unwrap();
        self.files.insert(file_path.canonicalize()?);
        self.dirs.insert(parent.into(), file_path);
        Ok(())
    }

    pub fn clear(&mut self) {
        let mut paths = self.notifier.paths_mut();
        for x in self.dirs.keys() {
            paths.remove(x).unwrap()
        }
        paths.commit().unwrap();
        self.dirs.clear();
    }
    pub fn wait_for_change(&mut self) {
        loop {
            let res = self.rx.recv().unwrap();
            if let Ok(ev) = res
                && !ev.kind.is_access()
                && ev
                    .paths
                    .iter()
                    .find(|i| self.files.contains(Path::new(i)))
                    .is_some()
                && self.last_time.elapsed() >= DEBOUNCE_TIME
            {
                self.last_time = Instant::now();
                break;
            }
        }
    }

    // pub fn wait_for_changes<'this>(&'this self) -> impl std::iter::Iterator<Item = ()> {
    //     let debounce_time = Duration::from_millis(5);
    //     let mut last_time = Instant::now();
    //     self.rx.iter().filter_map(move |res| {
    //         if let Ok(ev) = res
    //             && !ev.kind.is_access()
    //             && ev
    //                 .paths
    //                 .iter()
    //                 .find(|i| self.files.contains(Path::new(i)))
    //                 .is_some()
    //             && last_time.elapsed() >= debounce_time
    //         {
    //             last_time = Instant::now();
    //             return Some(());
    //         }
    //         return None;
    //     })
    // }
}
