use serde::{Deserialize, Serialize};

/// Parametros de una tarea de Empresite: el indice de pagina a recorrer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EmpresiteParams {
    pub page: u32,
}
