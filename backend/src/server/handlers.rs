use axum::{Json, extract::Multipart, http::StatusCode};
use std::{
    fs::{self, File},
    io::Write,
    path::Component,
    rc::Rc,
};

use crate::{
    abstraction_graph::graph::{self, Graph},
    name_resolution::{self, file_util::get_java_files},
    parser,
    types::JavaFile,
};

pub async fn graph_construct_handler(
    multipart: Multipart,
) -> Result<axum::Json<Graph>, (StatusCode, String)> {
    let uuid = uuid::Uuid::now_v7().to_string();
    let mut filesink = RealFileSink::new();

    let path = extract_files(&uuid, multipart, &mut filesink).await?;

    let files = get_java_files(std::path::Path::new(&path))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // from this point on, very similar to the old main (check github
    // https://github.com/KyAnhVo/VisualizeJava/commit/cbce2d3443853f6e1859c3919c09b22e72604da9,
    // a lot of changes but mostly just move files around).

    // Construct AST
    let mut asts: Vec<Rc<JavaFile>> = vec![];
    for file in files.iter() {
        if file.ends_with("package-info.java") {
            continue;
        }
        if file.ends_with("module-info.java") {
            continue;
        }
        let src_str = fs::read_to_string(file).unwrap();
        match parser::parser::Parser::parse(&src_str, file) {
            Ok(ast) => asts.push(Rc::new(ast)),
            Err(_) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Parse failed for file {}", file.to_string_lossy()),
                ));
            }
        };
    }

    // Construct type index, resolved tree, and abstraction graph.
    let resolved_tree = name_resolution::resolve::Resolver::resolve(&asts);
    let graph = graph::Graph::from_trees(resolved_tree.as_ref());

    Ok(Json(graph))
}

trait FileSink {
    fn write_file(&mut self, file: &mut File, data: &[u8]) -> std::io::Result<()>;
}
pub struct RealFileSink;

impl RealFileSink {
    fn new() -> Self {
        Self
    }
}

impl FileSink for RealFileSink {
    fn write_file(&mut self, file: &mut File, data: &[u8]) -> std::io::Result<()> {
        file.write_all(data)
    }
}

/// From the client's multipart message, receive and write to /tmp/:uuid.
/// Reminder to remove the file after a while (or use systemd-tmpfiles to cleanup)
async fn extract_files(
    uuid: &str,
    mut multipart: Multipart,
    filesink: &mut impl FileSink,
) -> Result<String, (StatusCode, String)> {
    use std::fs::OpenOptions;
    use std::path::Path;

    let mut osdir = String::new();
    osdir.push_str("/tmp/");
    osdir.push_str(uuid);
    osdir.push_str("/");
    let basepath = Path::new(&osdir);

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let Some(filename) = field.file_name() else {
            continue;
        };
        if !filename.ends_with(".java") {
            continue;
        }

        let relpath = Path::new(filename);
        if !check_valid_component(relpath) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Does not accept absolute path".to_string(),
            ));
        }

        let path = basepath.join(relpath);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let data = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        filesink
            .write_file(&mut file, &data)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(osdir)
}

fn check_valid_component(path: &std::path::Path) -> bool {
    if path.is_absolute() {
        return false;
    }
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
}

#[cfg(test)]
mod test {
    use std::io::Read;

    use super::*;
    struct MockFileSink {
        pub buffer: Vec<u8>,
    }

    impl FileSink for MockFileSink {
        fn write_file(&mut self, _: &mut File, data: &[u8]) -> std::io::Result<()> {
            self.buffer = data.to_vec();
            Ok(())
        }
    }

    impl MockFileSink {
        fn compare_to_system(&self, path: &std::path::Path) -> bool {
            use std::fs::OpenOptions;
            let file = OpenOptions::new()
                .create(false)
                .read(true)
                .open(path)
                .unwrap();
            let bytes: Vec<u8> = file.bytes().map(|x| x.unwrap()).collect();

            bytes == self.buffer
        }
    }

    #[tokio::test]
    async fn test_extract_file() {
        use axum::body::Body;
        use axum::extract::FromRequest;
        use axum::http::{Request, header};

        let boundary = "test-boundary";
        let file_contents = b"class Foo {}";
        let body = format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"Foo.java\"\r\n\
             Content-Type: text/plain\r\n\
             \r\n\
             {}\r\n\
             --{boundary}--\r\n",
            std::str::from_utf8(file_contents).unwrap()
        );

        let request = Request::builder()
            .method("POST")
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap();
        let multipart = Multipart::from_request(request, &()).await.unwrap();

        let mut filesink = MockFileSink { buffer: vec![] };
        let uuid = "test-extract-file-uuid";
        let result = extract_files(uuid, multipart, &mut filesink).await;

        assert_eq!(result.unwrap(), "/tmp/test-extract-file-uuid/");
        assert_eq!(filesink.buffer, file_contents);
        // extract_files always creates the real (empty) file via OpenOptions before
        // delegating the write to FileSink; the mock never touches it, so the file
        // on disk should stay empty and diverge from the mock's captured buffer.
        assert!(
            !filesink
                .compare_to_system(std::path::Path::new("/tmp/test-extract-file-uuid/Foo.java"))
        );

        let _ = std::fs::remove_dir_all("/tmp/test-extract-file-uuid");
    }
}
