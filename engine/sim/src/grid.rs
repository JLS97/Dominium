//! Grid espacial de orden estable (regla 9): `MAX_CELLS`, `GridConfig`,
//! `CellId` y `Grid`. Es la única forma de buscar vecinos: counting sort
//! estable por (celda, fila), consulta por radio con contador de candidatos y
//! permutación por celda para `Table::permute_active`. Es estado derivado
//! (regla 6): se reconstruye cada tick desde las posiciones y no se guarda ni
//! entra en el hash.
// Contrato A: los cuerpos son marcadores pendientes, no usan sus parámetros
// y no leen los campos privados. `expect` obliga a P02-T05 a retirar estas
// líneas al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P02-T05")]
#![expect(dead_code, reason = "campos privados sin leer hasta P02-T05")]

use engine_core::math::Vec2;
use serde::{Deserialize, Serialize};

use crate::buffer::FixedVec;
use crate::table::Row;

/// Celdas como máximo: 2^20. `rebuild` recorre todas las celdas cada tick, así que el tamaño de celda de cada mapa se
/// elige con `benches/grid.rs` para que el coste quepa en el tick, también con 300 unidades; el grid de navegación del
/// hito 2 es otra estructura y su tamaño lo fija su spec tras la pregunta 16 del GDD.
pub const MAX_CELLS: u64 = 1 << 20;

/// Un grid uniforme sobre el rectángulo `origin .. origin + cell_size * (cells_x, cells_y)`. Lo fija el mapa (datos).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GridConfig {
    /// Esquina del rectángulo. Finito.
    pub origin: Vec2,
    /// Lado de cada celda. Finito y `> 0`.
    pub cell_size: f32,
    /// Celdas en `x`. `>= 1`.
    pub cells_x: u32,
    /// Celdas en `y`. `>= 1`; `cells_x as u64 * cells_y as u64 <= MAX_CELLS`, comprobado en `u64`.
    pub cells_y: u32,
}

/// `y * cells_x + x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CellId(pub u32);

/// Grid espacial de orden estable (celda, fila). Es estado derivado: se reconstruye cada tick desde las posiciones y no se
/// guarda. Con counting sort estable: `rebuild` es lineal en filas más celdas y no asigna.
pub struct Grid {
    /// La configuración, ya validada en `new`.
    config: GridConfig,
    /// Filas por celda, una entrada por celda.
    counts: FixedVec<u32>,
    /// Desplazamiento de la primera fila de cada celda en `rows`, una entrada por celda más la final.
    starts: FixedVec<u32>,
    /// Las filas indexadas, ordenadas por (celda, fila). Su capacidad es `max_rows`.
    rows: FixedVec<Row>,
}

impl Grid {
    /// Exige una `config` válida (pánico `configuración de grid inválida` si no). Asigna una vez para `max_rows` filas y
    /// todas las celdas.
    ///
    /// # Panics
    ///
    /// Pánico `configuración de grid inválida` si `config` incumple lo que fija `GridConfig`.
    #[must_use]
    pub fn new(config: GridConfig, max_rows: usize) -> Grid {
        todo!("P02-T05")
    }

    /// La configuración con la que se creó.
    #[must_use]
    pub fn config(&self) -> &GridConfig {
        todo!("P02-T05")
    }

    /// Recorta al borde: una coordenada fuera del rectángulo cae en la celda del borde más cercano (`-0.0` y los infinitos
    /// incluidos), y una coordenada NaN cae en la celda 0 de su eje. Determinista.
    #[must_use]
    pub fn cell_of(&self, position: Vec2) -> CellId {
        todo!("P02-T05")
    }

    /// Indexa las filas `0..positions.len()`. Exige `positions.len() <= max_rows` (pánico `scratch insuficiente` si no). No asigna.
    ///
    /// # Panics
    ///
    /// Pánico `scratch insuficiente` si `positions.len() > max_rows`.
    pub fn rebuild(&mut self, positions: &[Vec2]) {
        todo!("P02-T05")
    }

    /// Filas indexadas en el último `rebuild`.
    #[must_use]
    pub fn len(&self) -> usize {
        todo!("P02-T05")
    }

    /// Cierto si el último `rebuild` no indexó ninguna fila.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        todo!("P02-T05")
    }

    /// Las filas de la celda, en orden ascendente.
    #[must_use]
    pub fn rows_in_cell(&self, cell: CellId) -> &[Row] {
        todo!("P02-T05")
    }

    /// Los candidatos son las filas de las celdas del rectángulo de `cell_of(center - radius)` a `cell_of(center + radius)`,
    /// recortado al grid; se recorren en orden fijo (celdas por `y` y luego `x`, filas ascendentes dentro de cada celda) y
    /// se visitan las que están a distancia al cuadrado ≤ `radius * radius` de `center`. Con `radius` negativo o NaN no
    /// examina ninguna y devuelve 0. `positions` es el mismo slice que recibió `rebuild`. Devuelve el número de candidatos
    /// examinados, que es el contador de la regla 9. La propia fila del centro se visita si está a distancia 0: la salta quien
    /// llama. No asigna.
    pub fn for_each_in_radius(
        &self,
        center: Vec2,
        radius: f32,
        positions: &[Vec2],
        visit: impl FnMut(Row),
    ) -> u32 {
        todo!("P02-T05")
    }

    /// Vacía `out` y escribe la permutación (celda, fila) de las filas indexadas, para `Table::permute_active`. Exige
    /// capacidad para `len()`; si no, pánico `columna llena`.
    ///
    /// # Panics
    ///
    /// Pánico `columna llena` si `out` no tiene capacidad para `len()`.
    pub fn cell_order(&self, out: &mut FixedVec<Row>) {
        todo!("P02-T05")
    }
}
