use serde::Serialize;

/// Parametros de una tarea de Empresite: el indice de pagina a recorrer.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct EmpresiteParams {
    pub page: u32,
    // pub headless: bool,
    // pub browser_path: Option<&'a Path>,
    // pub profile_dir: Option<&'a Path>,
}

/// Convierte el texto de la actividad en un slug apto para la URL de Empresite:
/// mayusculas, sin acentos, espacios por guiones y solo caracteres aceptados en
/// una URL (alfanumericos, `-`, `_`, `.` y `~`).
pub fn activity_slug(query: &str) -> String {
    let mut slug = String::with_capacity(query.len());
    for ch in query.trim().to_uppercase().chars() {
        let ch = match ch {
            'Á' | 'À' | 'Ä' | 'Â' | 'Ã' | 'Å' => 'A',
            'É' | 'È' | 'Ë' | 'Ê' => 'E',
            'Í' | 'Ì' | 'Ï' | 'Î' => 'I',
            'Ó' | 'Ò' | 'Ö' | 'Ô' | 'Õ' => 'O',
            'Ú' | 'Ù' | 'Ü' | 'Û' => 'U',
            'Ç' => 'C',
            'Ñ' => 'N',
            other => other,
        };
        if ch.is_ascii_alphanumeric() || ch == '_' {
            slug.push(ch);
        } else if ch.is_whitespace() || ch == '-' {
            if !slug.ends_with('-') {
                slug.push('-');
            }
        }
    }
    slug.trim_matches('-').to_string()
}

/// Extrae el slug de actividad del path de una URL de Empresite, tal como
/// quedo tras las redirecciones. Empresite puede canonicalizar el nombre
/// (`MANTENIMIENTO` -> `MANTENIMIENTOS`) y esta funcion devuelve el que
/// realmente sirvio la pagina. Devuelve `None` si la URL no apunta a una
/// actividad.
pub fn activity_from_url(url: &str) -> Option<String> {
    let rest = url.split("/Actividad/").nth(1)?;
    let slug = rest.split(['/', '?']).next()?.trim();
    if slug.is_empty() {
        None
    } else {
        Some(slug.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_slug_acentos_y_espacios() {
        assert_eq!(
            activity_slug("  fontanería y calefacción "),
            "FONTANERIA-Y-CALEFACCION"
        );
    }

    #[test]
    fn test_activity_slug_elimina_especiales() {
        assert_eq!(activity_slug("café/bar (centro)"), "CAFEBAR-CENTRO");
    }

    #[test]
    fn test_activity_slug_mantiene_guion_y_bajo() {
        assert_eq!(activity_slug("auto_escuela-test"), "AUTO_ESCUELA-TEST");
    }

    #[test]
    fn test_activity_from_url_pagina_1() {
        assert_eq!(
            activity_from_url("https://empresite.eleconomista.es/Actividad/MANTENIMIENTOS/"),
            Some("MANTENIMIENTOS".to_string())
        );
    }

    #[test]
    fn test_activity_from_url_con_paginado() {
        assert_eq!(
            activity_from_url(
                "https://empresite.eleconomista.es/Actividad/MANTENIMIENTOS/PgNum-2/"
            ),
            Some("MANTENIMIENTOS".to_string())
        );
    }

    #[test]
    fn test_activity_from_url_con_filtros() {
        assert_eq!(
            activity_from_url(
                "https://empresite.eleconomista.es/Actividad/MANTENIMIENTOS/PgNum-2/\
                 ?testfiltros=1&emp_web=true"
            ),
            Some("MANTENIMIENTOS".to_string())
        );
    }

    #[test]
    fn test_activity_from_url_sin_actividad() {
        assert_eq!(
            activity_from_url("https://empresite.eleconomista.es/empresas-provincia/"),
            None
        );
    }
}
