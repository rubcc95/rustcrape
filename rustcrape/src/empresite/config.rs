use serde::Serialize;

/// Parametros de una tarea de Empresite: el indice de pagina a recorrer.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct EmpresiteParams {
    pub page: u32,
    // pub headless: bool,
    // pub browser_path: Option<&'a Path>,
    // pub profile_dir: Option<&'a Path>,
}
