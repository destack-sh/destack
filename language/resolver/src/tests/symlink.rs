#[cfg(target_os = "windows")]
use crate::tests::windows::get_dos_device_path;
#[cfg(target_os = "windows")]
use destack_source::PathExt;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::{Resolver, ResolverOptions};

#[derive(Debug, Clone, Copy)]
enum FileType {
    File,
    Directory,
}

#[allow(unused_variables)]
fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(
    original: P,
    link: Q,
    file_type: FileType,
) -> io::Result<()> {
    #[cfg(target_family = "unix")]
    {
        std::os::unix::fs::symlink(original, link)
    }

    #[cfg(target_os = "windows")]
    match file_type {
        // NOTE: original path should use `\` instead of `/` for relative paths
        //       otherwise the symlink will be broken and the test will fail with InvalidFilename error
        FileType::File => std::os::windows::fs::symlink_file(original.as_ref().normalize(), link),
        FileType::Directory => {
            std::os::windows::fs::symlink_dir(original.as_ref().normalize(), link)
        }
    }
    #[cfg(target_family = "wasm")]
    {
        Err(io::Error::new(io::ErrorKind::Other, "not supported"))
    }
}

fn init(dirname: &Path, temp_path: &Path) -> io::Result<()> {
    if temp_path.exists() {
        _ = fs::remove_dir_all(temp_path);
    }
    fs::create_dir(temp_path)?;
    symlink(
        dirname.join("../lib/index.js"),
        temp_path.join("test"),
        FileType::File,
    )?;
    symlink(
        dirname.join("../lib"),
        temp_path.join("test2"),
        FileType::Directory,
    )?;
    fs::remove_dir_all(temp_path)
}

fn create_symlinks(dirname: &Path, temp_path: &Path) -> io::Result<()> {
    fs::create_dir(temp_path)?;
    symlink(
        dirname.join("../lib/index.js").canonicalize()?,
        temp_path.join("index.js"),
        FileType::File,
    )?;
    symlink(
        dirname.join("../lib").canonicalize().unwrap(),
        temp_path.join("lib"),
        FileType::Directory,
    )?;
    symlink(
        dirname.join("..").canonicalize().unwrap(),
        temp_path.join("this"),
        FileType::Directory,
    )?;
    symlink(
        temp_path.join("this"),
        temp_path.join("that"),
        FileType::Directory,
    )?;
    symlink(
        Path::new("../../lib/index.js"),
        temp_path.join("node.relative.js"),
        FileType::File,
    )?;
    symlink(
        Path::new("./node.relative.js"),
        temp_path.join("node.relative.sym.js"),
        FileType::File,
    )?;

    #[cfg(target_os = "windows")]
    {
        // Ideally we should point to a Volume that does not have a drive letter.
        // However, it's not trivial to create a Volume in CI environment.
        // Here we are just picking up any Volume, as resolver itself is not calling `fs::canonicalize`,
        // which potentially can resolve the Volume GUID into driver letter whenever possible.
        let dos_device_temp_path = get_dos_device_path(temp_path).unwrap();
        symlink(
            dos_device_temp_path.join(r"..\..\lib"),
            temp_path.join("device_path_lib"),
            FileType::Directory,
        )?;
        symlink(
            dos_device_temp_path.join(r"..\..\lib\index.js"),
            temp_path.join("device_path_index.js"),
            FileType::File,
        )?;
    }

    Ok(())
}

fn cleanup_symlinks(temp_path: &Path) {
    _ = fs::remove_dir_all(temp_path);
}

fn remove_directory_symlink(path: &Path) {
    _ = fs::remove_dir(path);
    _ = fs::remove_file(path);
}

struct SymlinkFixturePaths {
    root: PathBuf,
    temp_path: PathBuf,
}

/// Prepares symlinks for the test.
/// Specify a different `temp_path_segment` for each test to avoid conflicts when tests are executed concurrently.
/// Returns `Ok(None)` if the symlink fixtures cannot be created at all (usually due to a lack of permission).
/// Returns `Ok(Some(_))` if the symlink fixtures are created successfully, or already exist.
/// Returns `Err(_)` if there is error creating the symlinks.
fn prepare_symlinks<P: AsRef<Path>>(
    temp_path_segment: P,
) -> io::Result<Option<SymlinkFixturePaths>> {
    let root = super::fixture_root().join("enhanced_resolve");
    let dirname = root.join("test");
    let temp_path = dirname.join(temp_path_segment.as_ref());
    if !temp_path.exists() {
        if let Err(err) = init(&dirname, &temp_path) {
            println!(
                "Skipped test: Failed to create symlinks. You may need administrator privileges. Error: {err}"
            );
            return Ok(None);
        }
        if let Err(err) = create_symlinks(&dirname, &temp_path) {
            cleanup_symlinks(&temp_path);
            return Err(err);
        }
    }

    Ok(Some(SymlinkFixturePaths { root, temp_path }))
}

/// Test symlink resolution behavior.
#[test]
#[cfg_attr(target_family = "wasm", ignore)]
fn test_symlinks_resolution() {
    let Some(SymlinkFixturePaths { root, temp_path }) = prepare_symlinks("temp").unwrap() else {
        return;
    };
    let resolver_without_symlinks = Resolver::for_tests(ResolverOptions {
        canonicalize_symlinks: false,
        ..ResolverOptions::default()
    });
    let resolver_with_symlinks = Resolver::for_tests(ResolverOptions::default());

    #[rustfmt::skip]
    let pass = [
        ("with a symlink to a file", temp_path.clone(), "./index.js"),
        ("with a relative symlink to a file", temp_path.clone(), "./node.relative.js"),
        ("with a relative symlink to a symlink to a file", temp_path.clone(), "./node.relative.sym.js"),
        ("with a symlink to a directory 1", temp_path.clone(), "./lib/index.js"),
        ("with a symlink to a directory 2", temp_path.clone(), "./this/lib/index.js"),
        ("with multiple symlinks in the path 1", temp_path.clone(), "./this/test/temp/index.js"),
        ("with multiple symlinks in the path 2", temp_path.clone(), "./this/test/temp/lib/index.js"),
        ("with multiple symlinks in the path 3", temp_path.clone(), "./this/test/temp/this/lib/index.js"),
        ("with a symlink to a directory 2 (chained)", temp_path.clone(), "./that/lib/index.js"),
        ("with multiple symlinks in the path 1 (chained)", temp_path.clone(), "./that/test/temp/index.js"),
        ("with multiple symlinks in the path 2 (chained)", temp_path.clone(), "./that/test/temp/lib/index.js"),
        ("with multiple symlinks in the path 3 (chained)", temp_path.clone(), "./that/test/temp/that/lib/index.js"),
        ("with symlinked directory as context 1", temp_path.join( "lib"), "./index.js"),
        ("with symlinked directory as context 2", temp_path.join( "this"), "./lib/index.js"),
        ("with symlinked directory as context and in path", temp_path.join( "this"), "./test/temp/lib/index.js"),
        ("with symlinked directory in context path", temp_path.join( "this/lib"), "./index.js"),
        ("with symlinked directory in context path and symlinked file", temp_path.join( "this/test"), "./temp/index.js"),
        ("with symlinked directory in context path and symlinked directory", temp_path.join( "this/test"), "./temp/lib/index.js"),
        ("with symlinked directory as context 2 (chained)", temp_path.join( "that"), "./lib/index.js"),
        ("with symlinked directory as context and in path (chained)", temp_path.join( "that"), "./test/temp/lib/index.js"),
        ("with symlinked directory in context path (chained)", temp_path.join( "that/lib"), "./index.js"),
        ("with symlinked directory in context path and symlinked file (chained)", temp_path.join( "that/test"), "./temp/index.js"),
        ("with symlinked directory in context path and symlinked directory (chained)", temp_path.join( "that/test"), "./temp/lib/index.js")
    ];

    for (comment, path, request) in pass {
        let filename = resolver_with_symlinks
            .resolve_test_directory(&path, request)
            .map(|r| r.full_path());
        assert_eq!(filename, Ok(root.join("lib/index.js")), "{comment:?}");

        let resolved_path = resolver_without_symlinks
            .resolve_test_directory(&path, request)
            .map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(path.join(request)));
    }
}

/// Test circular symlink detection.
#[test]
fn test_symlinks_circular() {
    let Some(SymlinkFixturePaths { root: _, temp_path }) =
        prepare_symlinks("temp.test_circular_symlink").unwrap()
    else {
        return;
    };

    // create a circular symlink: link1 -> link2 -> link1
    let link1_path = temp_path.join("link1");
    let link2_path = temp_path.join("link2");

    if symlink(&link2_path, &link1_path, FileType::File).is_err() {
        // skip test if we can't create symlinks
        return;
    }
    if symlink(&link1_path, &link2_path, FileType::File).is_err() {
        // skip test if we can't create symlinks
        _ = fs::remove_file(&link1_path);
        return;
    }

    // should error due to circular symlink
    let resolver = Resolver::for_tests(ResolverOptions::default());
    let result = resolver.resolve_test_directory(&temp_path, "./link1");
    assert!(result.is_err());

    _ = fs::remove_file(&link1_path);
    _ = fs::remove_file(&link2_path);
}

/// Resolve self-reference exports inside a symlinked package directory.
#[test]
#[cfg_attr(target_family = "wasm", ignore)]
fn test_symlinks_self_reference_exports_resolution() {
    let Some(SymlinkFixturePaths { root: _, temp_path }) =
        prepare_symlinks("temp.test_symlink_self_reference_exports").unwrap()
    else {
        return;
    };

    let package_real_path = temp_path.join("selfpkg-real");
    let package_link_path = temp_path.join("selfpkg-link");
    let package_real_src_path = package_real_path.join("src");
    let package_link_src_path = package_link_path.join("src");
    let package_json_path = package_real_path.join("package.json");
    let package_index_path = package_real_src_path.join("index.js");
    let package_feature_path = package_real_src_path.join("feature.js");

    _ = fs::remove_dir_all(&package_real_path);
    remove_directory_symlink(&package_link_path);
    if fs::create_dir_all(&package_real_src_path).is_err() {
        return;
    }
    if fs::write(
        package_json_path,
        r#"{"name":"selfpkg","exports":{".":"./src/index.js","./feature":"./src/feature.js"}}"#,
    )
    .is_err()
    {
        _ = fs::remove_dir_all(&package_real_path);
        return;
    }
    if fs::write(&package_index_path, "export {};").is_err()
        || fs::write(&package_feature_path, "export {};").is_err()
    {
        _ = fs::remove_dir_all(&package_real_path);
        return;
    }
    if symlink(&package_real_path, &package_link_path, FileType::Directory).is_err() {
        _ = fs::remove_dir_all(&package_real_path);
        return;
    }

    let resolver_without_symlinks = Resolver::for_tests(ResolverOptions {
        canonicalize_symlinks: false,
        ..ResolverOptions::default()
    });
    let resolver_with_symlinks = Resolver::for_tests(ResolverOptions::default());

    let link_root_resolution = resolver_without_symlinks
        .resolve_test_directory(&package_link_src_path, "selfpkg")
        .map(|r| r.full_path());
    assert_eq!(
        link_root_resolution,
        Ok(package_link_src_path.join("index.js"))
    );
    let link_feature_resolution = resolver_without_symlinks
        .resolve_test_directory(&package_link_src_path, "selfpkg/feature")
        .map(|r| r.full_path());
    assert_eq!(
        link_feature_resolution,
        Ok(package_link_src_path.join("feature.js"))
    );

    let real_root_resolution = resolver_with_symlinks
        .resolve_test_directory(&package_link_src_path, "selfpkg")
        .map(|r| r.full_path());
    assert_eq!(
        real_root_resolution,
        Ok(package_real_src_path.join("index.js"))
    );
    let real_feature_resolution = resolver_with_symlinks
        .resolve_test_directory(&package_link_src_path, "selfpkg/feature")
        .map(|r| r.full_path());
    assert_eq!(
        real_feature_resolution,
        Ok(package_real_src_path.join("feature.js"))
    );

    remove_directory_symlink(&package_link_path);
    _ = fs::remove_dir_all(&package_real_path);
}

/// Resolve imports mappings from a symlinked package root.
#[test]
#[cfg_attr(target_family = "wasm", ignore)]
fn test_symlinks_package_imports_resolution() {
    let Some(SymlinkFixturePaths { root: _, temp_path }) =
        prepare_symlinks("temp.test_symlink_package_imports").unwrap()
    else {
        return;
    };

    let package_real_path = temp_path.join("imports-real");
    let package_link_path = temp_path.join("imports-link");
    let package_real_src_path = package_real_path.join("src");
    let package_link_src_path = package_link_path.join("src");
    let package_json_path = package_real_path.join("package.json");
    let package_self_path = package_real_src_path.join("self.js");

    _ = fs::remove_dir_all(&package_real_path);
    remove_directory_symlink(&package_link_path);
    if fs::create_dir_all(&package_real_src_path).is_err() {
        return;
    }
    if fs::write(
        package_json_path,
        r##"{"name":"imports-pkg","imports":{"#self":"./src/self.js"}}"##,
    )
    .is_err()
    {
        _ = fs::remove_dir_all(&package_real_path);
        return;
    }
    if fs::write(&package_self_path, "export {};").is_err() {
        _ = fs::remove_dir_all(&package_real_path);
        return;
    }
    if symlink(&package_real_path, &package_link_path, FileType::Directory).is_err() {
        _ = fs::remove_dir_all(&package_real_path);
        return;
    }

    let resolver_without_symlinks = Resolver::for_tests(ResolverOptions {
        canonicalize_symlinks: false,
        ..ResolverOptions::default()
    });
    let resolver_with_symlinks = Resolver::for_tests(ResolverOptions::default());

    let link_resolution = resolver_without_symlinks
        .resolve_test_directory(&package_link_src_path, "#self")
        .map(|r| r.full_path());
    assert_eq!(link_resolution, Ok(package_link_src_path.join("self.js")));

    let real_resolution = resolver_with_symlinks
        .resolve_test_directory(&package_link_src_path, "#self")
        .map(|r| r.full_path());
    assert_eq!(real_resolution, Ok(package_real_src_path.join("self.js")));

    remove_directory_symlink(&package_link_path);
    _ = fs::remove_dir_all(&package_real_path);
}
