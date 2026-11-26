use std::fs;
use std::path::Path;

pub fn make_room(directory: &Path, name: &str, suffix: &str, keep: usize) {
    if keep < 2 {
        return;
    }
    let numbered = |n: usize| directory.join(format!("{name}.{n}{suffix}"));

    let _ = fs::remove_file(numbered(keep - 1));
    for n in (1..keep - 1).rev() {
        let from = numbered(n);
        if from.exists() {
            let _ = fs::rename(from, numbered(n + 1));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::make_room;

    fn write(path: &std::path::Path, contents: &str) {
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn shifts_slots_up_and_drops_the_oldest() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        write(&path.join("app.1.ini"), "one");
        write(&path.join("app.2.ini"), "two");

        make_room(path, "app", ".ini", 10);

        assert!(!path.join("app.1.ini").exists());
        assert_eq!(fs::read_to_string(path.join("app.2.ini")).unwrap(), "one");
        assert_eq!(fs::read_to_string(path.join("app.3.ini")).unwrap(), "two");
    }

    #[test]
    fn drops_the_oldest_when_full() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        for n in 1..=9 {
            write(&path.join(format!("app.{n}.ini")), &n.to_string());
        }

        make_room(path, "app", ".ini", 10);

        assert_eq!(fs::read_to_string(path.join("app.2.ini")).unwrap(), "1");
        assert_eq!(fs::read_to_string(path.join("app.9.ini")).unwrap(), "8");
        assert!(!path.join("app.1.ini").exists());
        assert!(path.join("app.9.ini").exists());
    }

    #[test]
    fn ignores_gaps() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        write(&path.join("app.1.ini"), "one");
        write(&path.join("app.3.ini"), "three");

        make_room(path, "app", ".ini", 10);

        assert!(!path.join("app.1.ini").exists());
        assert_eq!(fs::read_to_string(path.join("app.2.ini")).unwrap(), "one");
        assert_eq!(fs::read_to_string(path.join("app.4.ini")).unwrap(), "three");
    }
}
