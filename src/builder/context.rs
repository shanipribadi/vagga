use std::io::{FileNotFound};
use std::io::ALL_PERMISSIONS;
use std::io::fs::{mkdir_recursive, mkdir, rmdir_recursive};
use std::io::fs::PathExtensions;
use std::collections::{TreeMap, TreeSet};

use container::mount::{bind_mount, unmount};
use config::Container;


pub struct BuildContext {
    pub container_name: String,
    pub container_config: Container,
    ensure_dirs: TreeSet<Path>,
    empty_dirs: TreeSet<Path>,
    remove_dirs: TreeSet<Path>,
    cache_dirs: TreeMap<Path, String>,
}

impl BuildContext {
    pub fn new(name: String, container: Container) -> BuildContext {
        return BuildContext {
            container_name: name,
            container_config: container,
            ensure_dirs: vec!(
                Path::new("proc"),
                Path::new("sys"),
                Path::new("dev"),
                Path::new("work"),
                Path::new("tmp"),
                ).into_iter().collect(),
            empty_dirs: vec!(
                Path::new("tmp"),
                Path::new("var/tmp"),
                ).into_iter().collect(),
            remove_dirs: vec!(
                ).into_iter().collect(),
            cache_dirs: vec!(
                ).into_iter().collect(),
        };
    }

    pub fn add_cache_dir(&mut self, path: Path, name: String)
        -> Result<(), String>
    {
        assert!(path.is_absolute());
        let path = path.path_relative_from(&Path::new("/")).unwrap();
        if self.cache_dirs.insert(path.clone(), name.clone()) {
            let cache_dir = Path::new("/vagga/cache").join(name.as_slice());
            if !cache_dir.exists() {
                try!(mkdir(&cache_dir, ALL_PERMISSIONS)
                     .map_err(|e| format!("Error creating cache dir: {}", e)));
            }
            let path = Path::new("/vagga/root").join(path);
            try!(empty_dir(&path));
            try!(bind_mount(&cache_dir, &path));
        }
        return Ok(());
    }

    pub fn add_remove_dir(&mut self, path: Path) {
        assert!(path.is_absolute());
        let path = path.path_relative_from(&Path::new("/")).unwrap();
        self.remove_dirs.insert(path);
    }

    pub fn add_empty_dir(&mut self, path: Path) {
        assert!(path.is_absolute());
        let path = path.path_relative_from(&Path::new("/")).unwrap();
        self.empty_dirs.insert(path);
    }

    pub fn add_ensure_dir(&mut self, path: Path) {
        assert!(path.is_absolute());
        let path = path.path_relative_from(&Path::new("/")).unwrap();
        self.ensure_dirs.insert(path);
    }

    pub fn finish(&self) -> Result<(), String> {
        let base = Path::new("/vagga/root");

        for (dir, _) in self.cache_dirs.rev_iter() {
            try!(unmount(&base.join(dir)));
        }

        for dir in self.remove_dirs.iter() {
            try!(rmdir_recursive(&base.join(dir))
                .map_err(|e| format!("Error removing dir: {}", e)));
        }

        for dir in self.empty_dirs.iter() {
            try!(empty_dir(&base.join(dir)));
        }

        for dir in self.ensure_dirs.iter() {
            try!(mkdir_recursive(&base.join(dir), ALL_PERMISSIONS)
                .map_err(|e| format!("Error creating dir: {}", e)));
        }

        return Ok(());
    }
}

fn empty_dir(dir: &Path) -> Result<(), String> {
    let perm = match dir.stat() {
        Ok(stat) => {
            try!(rmdir_recursive(dir)
                .map_err(|e| format!("Error removing dir: {}", e)));
            stat.perm
        }
        Err(ref e) if e.kind == FileNotFound => {
            ALL_PERMISSIONS
        }
        Err(e) => return Err(format!("Error stat: {}", e)),
    };
    try!(mkdir_recursive(dir, perm)
        .map_err(|e| format!("Error creating dir: {}", e)));
    return Ok(());
}
