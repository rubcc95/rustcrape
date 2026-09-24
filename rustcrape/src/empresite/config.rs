use serde::Serialize;

use crate::types::Province;

/// Segmento de path con la provincia (`provincia/{SLUG}/`) o cadena vacia si
/// no hay filtro por provincia. Empresite espera la provincia entre la
/// actividad y el `PgNum`: `/Actividad/{actividad}/provincia/{PROV}/PgNum-N/`.
pub fn province_path(province: Option<Province>) -> String {
    match province {
        Some(province) => format!("provincia/{}/", province.query_value()),
        None => String::new(),
    }
}

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

/// Accion a tomar tras cargar un listado, segun el activity que Empresite haya
/// decidido frente al solicitado.
#[derive(Debug, PartialEq, Eq)]
pub enum ActivityAction {
    /// No hay cambio: se puede parsear la pagina tal cual.
    Keep,
    /// Empresite renombro la actividad. Hay que fijar `canonical` y, si la
    /// redireccion perdio el `PgNum`, recargar la pagina con ese nombre.
    Renamed { canonical: String, reload: bool },
}

/// Decide si hay que corregir el activity a partir de la URL final servida.
///
/// Importante: debe llamarse con la respuesta real (200), nunca con el 429 de
/// bloqueo, porque una pagina bloqueada no redirige y no revela el nombre
/// canonico.
///
/// Cuando Empresite renombra la actividad la redireccion tambien descarta el
/// `PgNum` y la provincia, devolviendo siempre la pagina 1 sin filtro. Por eso
/// hay que recargar tanto si no estabamos en la primera pagina como si habia
/// una provincia configurada, incluso en la pagina 1.
pub fn activity_action(
    requested: &str,
    final_url: &str,
    page: u32,
    has_province: bool,
) -> ActivityAction {
    match activity_from_url(final_url) {
        Some(canonical) if canonical != requested => ActivityAction::Renamed {
            canonical,
            reload: page > 1 || has_province,
        },
        _ => ActivityAction::Keep,
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

    #[test]
    fn test_activity_action_sin_cambio() {
        assert_eq!(
            activity_action(
                "MANTENIMIENTO",
                "https://empresite.eleconomista.es/Actividad/MANTENIMIENTO/",
                1,
                false
            ),
            ActivityAction::Keep
        );
    }

    #[test]
    fn test_activity_action_429_no_redirige() {
        // El 429 devuelve la URL solicitada: no debe interpretarse como cambio.
        assert_eq!(
            activity_action(
                "MANTENIMIENTO",
                "https://empresite.eleconomista.es/Actividad/MANTENIMIENTO/PgNum-2/",
                2,
                false
            ),
            ActivityAction::Keep
        );
    }

    #[test]
    fn test_activity_action_renombrado_pagina_1() {
        assert_eq!(
            activity_action(
                "MANTENIMIENTO",
                "https://empresite.eleconomista.es/Actividad/MANTENIMIENTOS/",
                1,
                false
            ),
            ActivityAction::Renamed {
                canonical: "MANTENIMIENTOS".to_string(),
                reload: false,
            }
        );
    }

    #[test]
    fn test_activity_action_renombrado_pagina_2_recarga() {
        // La redireccion perdio el PgNum: hay que recargar la pagina 2.
        assert_eq!(
            activity_action(
                "MANTENIMIENTO",
                "https://empresite.eleconomista.es/Actividad/MANTENIMIENTOS/",
                2,
                false
            ),
            ActivityAction::Renamed {
                canonical: "MANTENIMIENTOS".to_string(),
                reload: true,
            }
        );
    }

    #[test]
    fn test_activity_action_url_sin_actividad() {
        assert_eq!(
            activity_action(
                "MANTENIMIENTO",
                "https://empresite.eleconomista.es/empresas-provincia/",
                2,
                false
            ),
            ActivityAction::Keep
        );
    }

    #[test]
    fn test_activity_action_renombrado_con_provincia_pagina_1_recarga() {
        // La redireccion perdio la provincia: hay que recargar la pagina 1.
        assert_eq!(
            activity_action(
                "MANTENIMIENTO",
                "https://empresite.eleconomista.es/Actividad/MANTENIMIENTOS/",
                1,
                true
            ),
            ActivityAction::Renamed {
                canonical: "MANTENIMIENTOS".to_string(),
                reload: true,
            }
        );
    }

    #[test]
    fn test_activity_from_url_con_provincia() {
        assert_eq!(
            activity_from_url(
                "https://empresite.eleconomista.es/Actividad/MANTENIMIENTOS/provincia/MADRID/PgNum-2/"
            ),
            Some("MANTENIMIENTOS".to_string())
        );
    }

    #[test]
    fn test_province_path() {
        use crate::types::Province;

        assert_eq!(province_path(None), "");
        assert_eq!(province_path(Some(Province::Madrid)), "provincia/MADRID/");
        assert_eq!(
            province_path(Some(Province::SantaCruzDeTenerife)),
            "provincia/SANTA-CRUZ-TENERIFE/"
        );
        assert_eq!(province_path(Some(Province::Rioja)), "provincia/RIOJA/");
        assert_eq!(province_path(Some(Province::Palmas)), "provincia/PALMAS/");
    }
}
