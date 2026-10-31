//! Tests de aceptación de P01-T07: `data` salvo `of_bytes`.
//!
//! Cubren CA-15 y CA-16 de `specs/architecture/engine_core.md`. Solo usan la API
//! pública de `engine_core` y no los modifica el implementador.
//!
//! Cada test trabaja en su propia carpeta bajo `CARGO_TARGET_TMPDIR`, con un nombre
//! único por test: la vacía al empezar si existe y la crea con `std::fs`. Así los
//! tests corren en paralelo sin estorbarse y no dejan restos entre ejecuciones.
//!
//! Sobre los errores: nunca se compara el `to_string()` de `DataError::Io`, porque
//! Windows localiza el texto del sistema; se comparan variantes con `matches!`, la
//! ruta y `source.kind()`. En `Parse` se comprueban `line`, `column`, `path` y que
//! `message` no esté vacío, sin fijar su texto, que es de `ron`.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use engine_core::data::{
    DATA_EXTENSION, DataError, DataFile, DataHash, hash_data_tree, list_data_files, load_ron,
    parse_ron,
};
use engine_core::hash::StateHasher;
use serde::Deserialize;

/// Contenido de `b.ron` en el árbol de CA-15.
const B_RON: &[u8] = b"(a: 1, b: 2)\n";
/// Contenido de `a/c.ron` en el árbol de CA-15.
const A_C_RON: &[u8] = b"(a: 3, b: 4)\n";
/// Contenido de `a/B.RON` en el árbol de CA-15. Distinto de los otros dos para que el
/// orden de los archivos importe en el hash.
const A_B_RON: &[u8] = b"(a: 5, b: 6)\n";
/// Contenido de `leeme.md`: no es `.ron`, así que no entra ni en el listado ni en el hash.
const LEEME_MD: &[u8] = b"# datos de prueba\n";
/// Contenido de `z.txt`: tampoco entra.
const Z_TXT: &[u8] = b"texto suelto\n";

/// Los tres archivos de datos de CA-15 en el orden que exige la spec: por los bytes
/// UTF-8 de `relative`, donde `B` (0x42) va antes que `c` (0x63) y `a/` antes que `b`.
const EXPECTED_DATA_FILES: [(&str, &[u8]); 3] =
    [("a/B.RON", A_B_RON), ("a/c.ron", A_C_RON), ("b.ron", B_RON)];

/// Struct local de CA-16. Los dos campos son obligatorios: `(a: 1, b: )` y `""` fallan.
#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Prueba {
    a: u32,
    b: u32,
}

/// Carpeta propia de un test bajo `CARGO_TARGET_TMPDIR`, vacía al salir de aquí.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("aceptacion_core_t07")
        .join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("vaciar la carpeta del test");
    }
    fs::create_dir_all(&dir).expect("crear la carpeta del test");
    dir
}

/// Escribe `relative` (con `/`) bajo `root`, creando las carpetas intermedias.
fn write_under(root: &Path, relative: &str, content: &[u8]) -> PathBuf {
    let path = relative
        .split('/')
        .fold(root.to_path_buf(), |acc, part| acc.join(part));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("crear las carpetas intermedias");
    }
    fs::write(&path, content).expect("escribir el archivo de prueba");
    path
}

/// Crea el árbol de CA-15 escribiendo los archivos en el orden dado.
fn write_tree(root: &Path, order: &[(&str, &[u8])]) {
    for (relative, content) in order {
        write_under(root, relative, content);
    }
}

/// Hash de árbol calculado a mano con `StateHasher`, siguiendo la doc de `hash_data_tree`:
/// por cada archivo en orden, los bytes UTF-8 de `relative`, un byte 0, la longitud del
/// contenido como `u64` little-endian y el contenido tal cual. `StateHasher` y
/// `DataHash::of_bytes` son xxh3 de 64 bits con semilla 0, así que el valor es comparable.
fn manual_tree_hash(files: &[(&str, &[u8])]) -> DataHash {
    let mut hasher = StateHasher::new();
    for (relative, content) in files {
        hasher.write(relative.as_bytes());
        hasher.write_u8(0);
        hasher.write_u64(u64::try_from(content.len()).expect("la longitud cabe en u64"));
        hasher.write(content);
    }
    DataHash(hasher.finish().0)
}

/// `hash_data_tree` sin el `Result`, con la ruta en el mensaje si falla.
fn tree_hash(root: &Path) -> DataHash {
    hash_data_tree(root).unwrap_or_else(|error| panic!("hash de `{}`: {error:?}", root.display()))
}

/// Comprueba la lista de CA-15: tres entradas, en orden, con `relative` usando `/` y
/// `path` apuntando a un archivo real con el contenido escrito.
fn assert_listing(root: &Path, files: &[DataFile]) {
    assert_eq!(
        files.len(),
        EXPECTED_DATA_FILES.len(),
        "solo los tres `.ron` entran en el listado: {files:?}"
    );
    for (file, (relative, content)) in files.iter().zip(EXPECTED_DATA_FILES) {
        assert_eq!(
            file.relative, relative,
            "orden por bytes UTF-8 de `relative`"
        );
        assert!(
            !file.relative.contains('\\'),
            "`relative` usa `/` en cualquier plataforma: {:?}",
            file.relative
        );
        // `path` es `root.join(relative)` con los separadores del sistema. `Path::ends_with`
        // compara componentes, así que no depende de cómo se haya normalizado `root`.
        let relative_components = relative
            .split('/')
            .fold(PathBuf::new(), |acc, part| acc.join(part));
        assert!(
            file.path.ends_with(&relative_components),
            "`path` `{}` termina en los componentes de `relative` {relative:?}",
            file.path.display()
        );
        assert!(
            file.path.starts_with(root),
            "`path` `{}` cuelga de la raíz `{}`",
            file.path.display(),
            root.display()
        );
        assert!(
            file.path.is_file(),
            "`path` `{}` es un archivo regular",
            file.path.display()
        );
        assert_eq!(
            fs::read(&file.path).expect("leer el archivo listado"),
            content,
            "`path` abre el archivo correcto"
        );
    }
}

/// Las mutaciones de CA-15 que deben cambiar el hash, cada una sobre una copia limpia
/// del árbol, y la que no debe cambiarlo.
fn assert_hash_sensitivity(base: DataHash) {
    // Cambia un byte de `b.ron`.
    let root = fresh_dir("ca_15_byte_b_ron");
    write_tree(&root, &EXPECTED_DATA_FILES);
    let mut changed = B_RON.to_vec();
    changed[B_RON.len() - 2] ^= 0x01;
    write_under(&root, "b.ron", &changed);
    assert_ne!(
        tree_hash(&root),
        base,
        "un byte distinto en `b.ron` cambia el hash"
    );

    // `b.ron` pasa a llamarse `bb.ron`, con el mismo contenido.
    let root = fresh_dir("ca_15_rename_b_ron");
    write_tree(&root, &EXPECTED_DATA_FILES);
    fs::rename(root.join("b.ron"), root.join("bb.ron")).expect("renombrar `b.ron`");
    assert_ne!(
        tree_hash(&root),
        base,
        "renombrar `b.ron` a `bb.ron` cambia el hash"
    );

    // Se añade `d.ron`.
    let root = fresh_dir("ca_15_add_d_ron");
    write_tree(&root, &EXPECTED_DATA_FILES);
    write_under(&root, "d.ron", b"(a: 7, b: 8)\n");
    assert_ne!(tree_hash(&root), base, "añadir `d.ron` cambia el hash");

    // Cambia `leeme.md`: no es `.ron`, el hash no se mueve.
    let root = fresh_dir("ca_15_leeme_md");
    write_tree(&root, &EXPECTED_DATA_FILES);
    write_under(&root, "leeme.md", LEEME_MD);
    write_under(&root, "z.txt", Z_TXT);
    assert_eq!(
        tree_hash(&root),
        base,
        "con `leeme.md` y `z.txt` el hash es el mismo"
    );
    write_under(
        &root,
        "leeme.md",
        b"# otro texto, mucho mas largo que el anterior\n",
    );
    assert_eq!(
        tree_hash(&root),
        base,
        "cambiar `leeme.md` no cambia el hash"
    );
}

/// CA-15: En una carpeta bajo `CARGO_TARGET_TMPDIR` con `b.ron`, `a/c.ron`, `a/B.RON`,
/// `leeme.md` y `z.txt`: `list_data_files` devuelve exactamente tres entradas en el orden
/// `a/B.RON`, `a/c.ron`, `b.ron`, con `relative` usando `/`; `hash_data_tree` da lo mismo
/// si los archivos se crean en otro orden; cambia si cambia un byte de `b.ron`, si `b.ron`
/// pasa a llamarse `bb.ron` o si se añade `d.ron`; no cambia si cambia `leeme.md`; una
/// carpeta vacía da `DataHash::of_bytes(&[])`; una carpeta inexistente da `DataError::Io`
/// con esa ruta; y el valor es igual al calculado a mano con un `StateHasher` siguiendo
/// la definición de la API.
#[test]
fn ca_15_hash_de_datos_ordenado_por_ruta_y_solo_ron() {
    assert_eq!(
        DATA_EXTENSION, "ron",
        "la extensión de datos fijada por la spec"
    );

    // Árbol de referencia, escrito en el orden del enunciado.
    let root = fresh_dir("ca_15_arbol");
    write_tree(
        &root,
        &[
            ("b.ron", B_RON),
            ("a/c.ron", A_C_RON),
            ("a/B.RON", A_B_RON),
            ("leeme.md", LEEME_MD),
            ("z.txt", Z_TXT),
        ],
    );
    let files = list_data_files(&root).expect("listar el árbol de prueba");
    assert_listing(&root, &files);

    let base = tree_hash(&root);
    assert_eq!(
        base,
        manual_tree_hash(&EXPECTED_DATA_FILES),
        "el hash coincide con el calculado a mano según la definición de la API"
    );
    assert_ne!(
        base,
        DataHash::of_bytes(&[]),
        "un árbol con archivos no da el hash de la secuencia vacía"
    );

    // El mismo árbol escrito en otro orden da el mismo hash y la misma lista.
    let shuffled = fresh_dir("ca_15_otro_orden");
    write_tree(
        &shuffled,
        &[
            ("z.txt", Z_TXT),
            ("a/B.RON", A_B_RON),
            ("leeme.md", LEEME_MD),
            ("b.ron", B_RON),
            ("a/c.ron", A_C_RON),
        ],
    );
    assert_eq!(
        tree_hash(&shuffled),
        base,
        "el orden de creación no cambia el hash"
    );
    let shuffled_files = list_data_files(&shuffled).expect("listar el árbol en otro orden");
    assert_eq!(
        shuffled_files
            .iter()
            .map(|f| f.relative.as_str())
            .collect::<Vec<_>>(),
        files
            .iter()
            .map(|f| f.relative.as_str())
            .collect::<Vec<_>>(),
        "el orden del listado no depende del sistema de archivos"
    );

    assert_hash_sensitivity(base);

    // Carpeta vacía: lista vacía y hash de la secuencia vacía.
    let empty = fresh_dir("ca_15_vacia");
    assert!(
        list_data_files(&empty)
            .expect("listar una carpeta vacía")
            .is_empty(),
        "una carpeta vacía no lista nada"
    );
    assert_eq!(
        tree_hash(&empty),
        DataHash::of_bytes(&[]),
        "una carpeta vacía da el hash de la secuencia vacía"
    );

    // Carpeta inexistente: `Io` con esa ruta, tanto al listar como al hashear.
    let missing = fresh_dir("ca_15_inexistente").join("no_existe");
    assert!(!missing.exists(), "la carpeta de prueba no debe existir");
    match list_data_files(&missing) {
        Err(DataError::Io { path, source }) => {
            assert_eq!(path, missing, "`Io` lleva la ruta que falló");
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("una carpeta inexistente da `Io`, no {other:?}"),
    }
    match hash_data_tree(&missing) {
        Err(DataError::Io { path, .. }) => assert_eq!(path, missing),
        other => panic!("una carpeta inexistente da `Io` al hashear, no {other:?}"),
    }
}

/// CA-15 (solo Windows): un archivo cuyo nombre se construye con
/// `OsStringExt::from_wide(&[0x61, 0xD800])` más `.ron` hace que `list_data_files`
/// devuelva `InvalidPath`. `0xD800` es un sustituto UTF-16 sin pareja: NTFS lo acepta,
/// pero no existe en UTF-8.
#[cfg(windows)]
#[test]
fn ca_15_nombre_no_utf8_es_invalid_path() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let root = fresh_dir("ca_15_no_utf8");
    // Un archivo válido al lado, para que el error no venga de una carpeta vacía.
    write_under(&root, "ok.ron", B_RON);

    let mut name = OsString::from_wide(&[0x61, 0xD800]);
    name.push(".ron");
    assert!(
        name.to_str().is_none(),
        "el nombre construido no debe ser UTF-8 válido"
    );
    let bad_path = root.join(&name);
    if let Err(error) = fs::write(&bad_path, b"()") {
        panic!(
            "el sistema de archivos no permitió crear el archivo con nombre no UTF-8 \
             `{}`: {error}. El test no se salta: hay que ejecutarlo sobre NTFS.",
            bad_path.display()
        );
    }

    match list_data_files(&root) {
        Err(DataError::InvalidPath { path }) => {
            assert_eq!(
                path.file_name(),
                Some(name.as_os_str()),
                "`InvalidPath` lleva la ruta con el nombre que no se pudo convertir"
            );
        }
        other => panic!("un nombre no UTF-8 da `InvalidPath`, no {other:?}"),
    }
    assert!(
        matches!(hash_data_tree(&root), Err(DataError::InvalidPath { .. })),
        "`hash_data_tree` propaga el `InvalidPath` del listado"
    );
}

/// Comprueba que `result` es `Parse` con la ruta esperada, devolviendo `(line, column)`.
fn expect_parse_error(
    result: Result<Prueba, DataError>,
    expected_path: Option<&Path>,
) -> (u32, u32) {
    match result {
        Err(DataError::Parse {
            path,
            line,
            column,
            message,
        }) => {
            assert_eq!(
                path.as_deref(),
                expected_path,
                "`Parse` lleva la ruta de origen"
            );
            assert!(line >= 1, "`line` empieza en 1");
            assert!(column >= 1, "`column` empieza en 1");
            assert!(
                !message.is_empty(),
                "`message` lleva el texto del analizador"
            );
            (line, column)
        }
        other => panic!("se esperaba `DataError::Parse`, no {other:?}"),
    }
}

/// CA-16: `parse_ron::<Prueba>("(a: 1, b: )")` devuelve `DataError::Parse` con `line == 1`,
/// `column > 0` y `path == None`; `parse_ron::<Prueba>("")` devuelve `Parse`; `load_ron` del
/// texto malo en un archivo bajo `CARGO_TARGET_TMPDIR` devuelve `Parse` con
/// `path == Some(ruta)`; `load_ron` de una ruta inexistente devuelve `Io` con esa ruta;
/// `load_ron` de un archivo con bytes que no son UTF-8 devuelve `Io` con
/// `source.kind() == InvalidData`; y un archivo que empieza por la marca de orden de bytes
/// UTF-8 seguida de `(a: 1, b: 2)` se carga bien.
#[test]
fn ca_16_errores_ron_con_linea_columna_y_ruta() {
    const BAD_TEXT: &str = "(a: 1, b: )";
    const GOOD_TEXT: &str = "(a: 1, b: 2)";
    const EXPECTED: Prueba = Prueba { a: 1, b: 2 };

    // Texto bien formado: la base sobre la que tienen sentido los errores.
    assert_eq!(
        parse_ron::<Prueba>(GOOD_TEXT).expect("RON válido"),
        EXPECTED,
        "`parse_ron` deserializa con las opciones por defecto de `ron`"
    );

    // Texto malo: `Parse` en la línea 1, columna positiva, sin ruta.
    let (line, _column) = expect_parse_error(parse_ron::<Prueba>(BAD_TEXT), None);
    assert_eq!(line, 1, "el error de `(a: 1, b: )` está en la línea 1");

    // Texto vacío: `Parse`, sin ruta.
    expect_parse_error(parse_ron::<Prueba>(""), None);

    // El mismo fallo en la segunda línea se sitúa en la línea 2: `line` cuenta líneas de verdad.
    let (line, _column) = expect_parse_error(parse_ron::<Prueba>("(a: 1,\n b: )"), None);
    assert_eq!(
        line, 2,
        "el error tras un salto de línea está en la línea 2"
    );

    // `parse_ron` quita la marca de orden de bytes inicial.
    assert_eq!(
        parse_ron::<Prueba>("\u{FEFF}(a: 1, b: 2)").expect("RON válido con BOM"),
        EXPECTED,
        "`parse_ron` acepta la marca de orden de bytes UTF-8 inicial"
    );

    let root = fresh_dir("ca_16_archivos");

    // Archivo con el texto malo: `Parse` con `path == Some(ruta)` y la misma posición.
    let bad_file = write_under(&root, "malo.ron", BAD_TEXT.as_bytes());
    let (file_line, _column) = expect_parse_error(load_ron::<Prueba>(&bad_file), Some(&bad_file));
    assert_eq!(
        file_line, 1,
        "la posición no cambia por venir de un archivo"
    );

    // Ruta inexistente: `Io` con esa ruta.
    let missing = root.join("no_existe.ron");
    assert!(!missing.exists(), "el archivo de prueba no debe existir");
    match load_ron::<Prueba>(&missing) {
        Err(DataError::Io { path, source }) => {
            assert_eq!(path, missing, "`Io` lleva la ruta que falló");
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("una ruta inexistente da `Io`, no {other:?}"),
    }

    // Contenido que no es UTF-8: `Io` con `InvalidData` y la ruta.
    let not_utf8 = write_under(&root, "no_utf8.ron", &[0xFF, 0xFE, 0x28]);
    match load_ron::<Prueba>(&not_utf8) {
        Err(DataError::Io { path, source }) => {
            assert_eq!(path, not_utf8, "`Io` lleva la ruta del archivo");
            assert_eq!(
                source.kind(),
                std::io::ErrorKind::InvalidData,
                "contenido no UTF-8 es `InvalidData`"
            );
        }
        other => panic!("contenido no UTF-8 da `Io`, no {other:?}"),
    }

    // Marca de orden de bytes `EF BB BF` seguida del texto bueno: se carga bien.
    let mut with_bom = vec![0xEF, 0xBB, 0xBF];
    with_bom.extend_from_slice(GOOD_TEXT.as_bytes());
    let bom_file = write_under(&root, "con_bom.ron", &with_bom);
    assert_eq!(
        load_ron::<Prueba>(&bom_file).expect("RON válido con BOM desde archivo"),
        EXPECTED,
        "`load_ron` quita la marca de orden de bytes inicial"
    );

    // La misma extensión que fija el contrato: los archivos de este test son datos.
    assert_eq!(bad_file.extension(), Some(OsStr::new(DATA_EXTENSION)));
}
