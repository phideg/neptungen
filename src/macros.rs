#[macro_export]
macro_rules! comp_as_str {
    ($c:expr) => {
        $c.as_os_str()
            .to_str()
            .with_context(|| format!("Couldn't read name of path component '{:?}'.", $c))
    };
}

#[macro_export]
macro_rules! last_path_comp {
    ($p:expr) => {
        $p.components()
            .last()
            .context("Invalid last path component")
    };
}

#[macro_export]
macro_rules! last_path_comp_as_str {
    ($p:expr) => {
        $p.components()
            .last()
            .and_then(|comp| comp.as_os_str().to_str())
            .context("Invalid last path component")
    };
}

#[cfg(test)]
mod tests {
    use anyhow::Context;

    #[test]
    fn test_comp_as_str() {
        let p = std::path::Path::new("/a");
        assert_eq!(comp_as_str!(p.components().last().unwrap()).unwrap(), "a");
    }

    #[test]
    fn test_last_path_comp() {
        let p = std::path::Path::new("/a/b");
        assert_eq!(comp_as_str!(last_path_comp!(p).unwrap()).unwrap(), "b");
    }

    #[test]
    fn test_last_path_comp_as_str() {
        let p = std::path::Path::new("/a/b");
        assert_eq!(last_path_comp_as_str!(p).unwrap(), "b");
    }
}
