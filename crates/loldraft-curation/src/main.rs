//! `loldraft-curate`: herramienta de curación de perfiles por fichero.
//!
//! No abre la base de datos ni habla con la app. Lee y escribe conjuntos de
//! perfiles validados para que una revisión pueda inspeccionarse antes de
//! importarla.

use loldraft_curation::{
    apply_identity, curate_inferred_cells, profile_cell_counts, profile_provenance_counts,
    same_curator, set_cell_counts, set_provenance_counts, CurateReport, ReviewSelection,
};
use loldraft_domain::ChampionKey;
use loldraft_engine::{taxonomy, ChampionProfile, ProfileProposalSet, ProfileSet, Provenance};
use std::collections::BTreeMap;
use std::fmt::{self, Write as _};
use std::path::Path;

const USAGE: &str = "\
loldraft-curate — curación de perfiles estratégicos

USO
  loldraft-curate <comando> [opciones]

COMANDOS
  from-proposals <propuestas.json> --out <perfiles.json>
      Expande una propuesta a un conjunto de perfiles. Todas las celdas
      declaradas quedan como `Inferred`; las ausentes, desconocidas.
  list <perfiles.json>
      Lista campeones, roles y cobertura de celdas.
  show <perfiles.json> <clave|slug|nombre>
      Muestra el perfil completo de un campeón.
  review <perfiles.json> --editor <n> --reviewer <n> --confidence <0..1> --out <salida.json>
      Convierte celdas `Inferred` conocidas en `Curated` y fija su confianza.
  approve <perfiles.json> --editor <n> --reviewer <n> --out <salida.json>
      Convierte celdas `Inferred` conocidas en `Curated` conservando su
      confianza. No rellena celdas desconocidas.
  check <perfiles.json> [--strict]
      Valida el conjunto y resume su cobertura y procedencia.

OPCIONES DE REVIEW/APPROVE
  --champion <clave>   Limita la revisión a esos campeones. Repetible o con
                       varias claves separadas por comas.
  --id <id>            Nuevo id del conjunto publicado.
  --version <versión>  Nueva versión del conjunto publicado.
  --out <salida.json>  Fichero de salida (obligatorio).

OTRAS OPCIONES
  --strict             `check` falla si quedan celdas inferidas.
  -h, --help           Muestra esta ayuda.
  -V, --version        Muestra la versión.

La herramienta nunca inventa celdas: solo cambia la procedencia de celdas que
ya existen. Las celdas desconocidas se quedan desconocidas. Si el editor y el
revisor son la misma persona, se avisa: `Curated` documenta una segunda
revisión.
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("error: {error}");
        let code = match error {
            CliError::Usage(_) => 2,
            CliError::Runtime(_) => 1,
        };
        std::process::exit(code);
    }
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Runtime(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Usage(message) | CliError::Runtime(message) => f.write_str(message),
        }
    }
}

fn usage(message: impl Into<String>) -> CliError {
    CliError::Usage(message.into())
}

fn runtime(message: impl Into<String>) -> CliError {
    CliError::Runtime(message.into())
}

fn run(args: &[String]) -> Result<(), CliError> {
    if args.is_empty() {
        print!("{USAGE}");
        return Ok(());
    }
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print!("{USAGE}");
        return Ok(());
    }
    if args[0] == "-V" || args[0] == "--version" {
        println!("loldraft-curate {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    match args[0].as_str() {
        "from-proposals" => from_proposals(&args[1..]),
        "list" => list(&args[1..]),
        "show" => show(&args[1..]),
        "review" => curate(&args[1..], CurationMode::Review),
        "approve" => curate(&args[1..], CurationMode::Approve),
        "check" => check(&args[1..]),
        other => Err(usage(format!(
            "comando desconocido `{other}`. Usa `loldraft-curate --help`."
        ))),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CurationMode {
    Review,
    Approve,
}

struct ParsedArgs {
    positionals: Vec<String>,
    flags: BTreeMap<String, Vec<String>>,
}

impl ParsedArgs {
    fn single(&self, name: &str) -> Result<Option<&str>, CliError> {
        match self.flags.get(name).map(Vec::as_slice) {
            None | Some([]) => Ok(None),
            Some([value]) => Ok(Some(value.as_str())),
            Some(_) => Err(usage(format!("`--{name}` solo admite un valor"))),
        }
    }

    fn required(&self, name: &str) -> Result<&str, CliError> {
        self.single(name)?
            .ok_or_else(|| usage(format!("falta la opción obligatoria `--{name}`")))
    }

    fn many(&self, name: &str) -> &[String] {
        self.flags.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    fn has(&self, name: &str) -> bool {
        self.flags.contains_key(name)
    }

    fn positionals(&self, expected: usize, syntax: &str) -> Result<(), CliError> {
        if self.positionals.len() == expected {
            Ok(())
        } else {
            Err(usage(format!(
                "se esperaban {expected} argumento(s): `loldraft-curate {syntax}`"
            )))
        }
    }
}

fn parse_args(
    args: &[String],
    allowed: &[&str],
    switches: &[&str],
) -> Result<ParsedArgs, CliError> {
    let mut positionals = Vec::new();
    let mut flags: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        if arg == "--" {
            positionals.extend(iter.by_ref().cloned());
            break;
        }
        if let Some(rest) = arg.strip_prefix("--") {
            let (name, inline) = match rest.split_once('=') {
                Some((name, value)) => (name, Some(value.to_string())),
                None => (rest, None),
            };
            if name.is_empty() {
                return Err(usage("opción vacía `--`"));
            }
            if !allowed.contains(&name) {
                return Err(usage(format!(
                    "opción desconocida `--{name}`. Usa `loldraft-curate --help`."
                )));
            }
            if switches.contains(&name) {
                if inline.is_some() {
                    return Err(usage(format!("`--{name}` no admite un valor")));
                }
                flags.entry(name.to_string()).or_default();
                continue;
            }
            let value = match inline {
                Some(value) => value,
                None => iter
                    .next()
                    .cloned()
                    .ok_or_else(|| usage(format!("`--{name}` requiere un valor")))?,
            };
            flags.entry(name.to_string()).or_default().push(value);
        } else if arg.starts_with('-') && arg.len() > 1 {
            return Err(usage(format!(
                "opción desconocida `{arg}`. Usa `loldraft-curate --help`."
            )));
        } else {
            positionals.push(arg.clone());
        }
    }

    Ok(ParsedArgs { positionals, flags })
}

fn from_proposals(args: &[String]) -> Result<(), CliError> {
    let parsed = parse_args(args, &["out"], &[])?;
    parsed.positionals(1, "from-proposals <propuestas.json> --out <perfiles.json>")?;
    let input = &parsed.positionals[0];
    let out = parsed.required("out")?;

    let text = read_text(input)?;
    let proposals = ProfileProposalSet::from_json(&text)
        .map_err(|error| runtime(format!("{input}: {error}")))?;
    let set = proposals
        .into_profile_set(taxonomy())
        .map_err(|error| runtime(format!("{input}: {error}")))?;
    if set.is_empty() {
        return Err(runtime(
            "la propuesta no contiene campeones; no se escribe un conjunto vacío",
        ));
    }
    write_set(out, &set)?;

    let counts = set_cell_counts(&set);
    println!("Conjunto inferido escrito en {out}");
    print_identity(&set);
    println!("  campeones: {}", set.len());
    println!("  roles declarados: {}", role_count(&set));
    println!(
        "  celdas conocidas: {} (inferidas: {}) · desconocidas: {}",
        counts.known(),
        counts.inferred,
        counts.unknown
    );
    println!(
        "Aviso: todas las celdas conocidas son `Inferred`; revisa el conjunto antes de publicarlo como curado."
    );
    Ok(())
}

fn list(args: &[String]) -> Result<(), CliError> {
    let parsed = parse_args(args, &[], &[])?;
    parsed.positionals(1, "list <perfiles.json>")?;
    let set = load_set(&parsed.positionals[0])?;

    println!(
        "{} v{} · {} campeones · taxonomía {}",
        set.id,
        set.version,
        set.len(),
        set.taxonomy_version
    );
    if set.is_empty() {
        println!("(sin campeones)");
        return Ok(());
    }

    for profile in set.champions.values() {
        let counts = profile_cell_counts(profile);
        let roles = profile
            .roles
            .iter()
            .map(|role| role.role.as_str())
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{:>4}  {:<20} {:<24} {:>3}/{:<3} celdas · {} inferidas · {} observadas · {} desconocidas",
            profile.key.0,
            profile.name,
            roles,
            counts.known(),
            counts.total(),
            counts.inferred,
            counts.observed,
            counts.unknown
        );
    }
    Ok(())
}

fn show(args: &[String]) -> Result<(), CliError> {
    let parsed = parse_args(args, &[], &[])?;
    parsed.positionals(2, "show <perfiles.json> <clave|slug|nombre>")?;
    let set = load_set(&parsed.positionals[0])?;
    let profile = find_profile(&set, &parsed.positionals[1])?;

    let mut out = String::new();
    writeln!(out, "{} ({}) · {}", profile.name, profile.key, profile.slug)
        .expect("escritura en memoria");
    writeln!(out, "Conjunto: {} v{}", set.id, set.version).expect("escritura en memoria");
    if let Some(patch) = &set.patch {
        writeln!(out, "Parche: {patch}").expect("escritura en memoria");
    }

    let counts = profile_cell_counts(profile);
    writeln!(
        out,
        "\nCeldas: {} conocidas de {} · {} observadas · {} inferidas · {} desconocidas",
        counts.known(),
        counts.total(),
        counts.observed,
        counts.inferred,
        counts.unknown
    )
    .expect("escritura en memoria");
    let provenance = profile_provenance_counts(profile);
    writeln!(
        out,
        "Procedencia: riot {} · curado {} · importado {} · inferido {} · desconocido {}",
        provenance.riot,
        provenance.curated,
        provenance.imported,
        provenance.inferred,
        provenance.unknown
    )
    .expect("escritura en memoria");

    out.push_str("\nRoles\n");
    for role in &profile.roles {
        writeln!(
            out,
            "  {:<8} {:>5}  {}",
            role.role.label_es(),
            percent(role.proficiency),
            describe_source(&role.source, role.confidence)
        )
        .expect("escritura en memoria");
    }

    match profile.damage_shares() {
        Some(shares) => {
            writeln!(
                out,
                "\nDaño\n  físico {} · mágico {} · verdadero {}",
                percent(shares.physical),
                percent(shares.magic),
                percent(shares.true_damage)
            )
            .expect("escritura en memoria");
        }
        None => {
            out.push_str("\nDaño\n  desconocido (no se asume 0)\n");
        }
    }

    let mut archetypes: Vec<_> = profile
        .archetypes
        .iter()
        .filter_map(|(id, rating)| rating.intensity.map(|value| (*id, value, rating)))
        .collect();
    archetypes.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.as_str().cmp(right.0.as_str()))
    });
    out.push_str("\nArquetipos\n");
    if archetypes.is_empty() {
        out.push_str("  (sin arquetipos conocidos)\n");
    }
    for (id, value, rating) in archetypes {
        let label = taxonomy()
            .archetype(id)
            .map(|entry| entry.label_es.as_str())
            .unwrap_or_else(|| id.as_str());
        let anchor = taxonomy().intensity_label(Some(value)).unwrap_or("");
        writeln!(
            out,
            "  {:<20} {:>5}  {:<12} {}",
            label,
            percent(value),
            anchor,
            describe_source(&rating.source, rating.confidence)
        )
        .expect("escritura en memoria");
    }

    let mut traits: Vec<_> = profile
        .traits
        .iter()
        .filter(|(id, _)| !id.is_damage_share())
        .filter_map(|(id, rating)| rating.value.map(|value| (*id, value, rating)))
        .collect();
    traits.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.as_str().cmp(right.0.as_str()))
    });
    out.push_str("\nTraits (sin daño)\n");
    if traits.is_empty() {
        out.push_str("  (sin traits conocidos)\n");
    }
    for (id, value, rating) in traits {
        let label = taxonomy()
            .trait_entry(id)
            .map(|entry| entry.label_es.as_str())
            .unwrap_or_else(|| id.as_str());
        writeln!(
            out,
            "  {:<20} {:>5}  {}",
            label,
            percent(value),
            describe_source(&rating.source, rating.confidence)
        )
        .expect("escritura en memoria");
    }

    let unknown_archetypes: Vec<&str> = profile
        .archetypes
        .iter()
        .filter(|(_, rating)| rating.intensity.is_none())
        .map(|(id, _)| {
            taxonomy()
                .archetype(*id)
                .map(|entry| entry.label_es.as_str())
                .unwrap_or_else(|| id.as_str())
        })
        .collect();
    let unknown_traits: Vec<&str> = profile
        .traits
        .iter()
        .filter(|(id, rating)| !id.is_damage_share() && rating.value.is_none())
        .map(|(id, _)| {
            taxonomy()
                .trait_entry(*id)
                .map(|entry| entry.label_es.as_str())
                .unwrap_or_else(|| id.as_str())
        })
        .collect();
    if !unknown_archetypes.is_empty() || !unknown_traits.is_empty() {
        writeln!(
            out,
            "\nDesconocidos\n  arquetipos: {}\n  traits: {}",
            list_or_none(&unknown_archetypes),
            list_or_none(&unknown_traits)
        )
        .expect("escritura en memoria");
    }

    print!("{out}");
    Ok(())
}

fn curate(args: &[String], mode: CurationMode) -> Result<(), CliError> {
    let allowed: &[&str] = match mode {
        CurationMode::Review => &[
            "editor",
            "reviewer",
            "confidence",
            "champion",
            "id",
            "version",
            "out",
        ],
        CurationMode::Approve => &["editor", "reviewer", "champion", "id", "version", "out"],
    };
    let syntax = match mode {
        CurationMode::Review => {
            "review <perfiles.json> --editor <n> --reviewer <n> --confidence <0..1> --out <salida.json>"
        }
        CurationMode::Approve => {
            "approve <perfiles.json> --editor <n> --reviewer <n> --out <salida.json>"
        }
    };
    let parsed = parse_args(args, allowed, &[])?;
    parsed.positionals(1, syntax)?;
    let input = &parsed.positionals[0];
    let out = parsed.required("out")?;
    let editor = parsed.required("editor")?;
    let reviewer = parsed.required("reviewer")?;
    let confidence = match mode {
        CurationMode::Review => {
            let raw = parsed.required("confidence")?;
            let value = raw
                .parse::<f32>()
                .map_err(|_| usage(format!("`--confidence` no es un número: {raw}")))?;
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(usage(format!(
                    "`--confidence` debe estar en 0..=1; recibido {raw}"
                )));
            }
            Some(value)
        }
        CurationMode::Approve => None,
    };

    let selection = ReviewSelection::only(parse_champion_keys(parsed.many("champion"))?);
    let mut set = load_set(input)?;
    if set.is_empty() {
        return Err(runtime(
            "el conjunto no contiene campeones; no hay nada que curar",
        ));
    }
    apply_identity(&mut set, parsed.single("id")?, parsed.single("version")?)
        .map_err(|error| usage(error.to_string()))?;

    let report = curate_inferred_cells(&mut set, &selection, editor, reviewer, confidence)
        .map_err(|error| runtime(error.to_string()))?;
    write_set(out, &set)?;

    match mode {
        CurationMode::Review => println!("Conjunto revisado escrito en {out}"),
        CurationMode::Approve => println!("Conjunto aprobado escrito en {out}"),
    }
    print_identity(&set);
    print_report(&report, mode);
    if same_curator(editor, reviewer) {
        println!(
            "Aviso: el editor y el revisor coinciden; `Curated` documenta una revisión por una segunda persona."
        );
    }
    if set.id.to_lowercase().contains("inferred") {
        println!(
            "Aviso: el id del conjunto sigue conteniendo `inferred`. Usa `--id` para publicar una identidad propia."
        );
    }
    Ok(())
}

fn check(args: &[String]) -> Result<(), CliError> {
    let parsed = parse_args(args, &["strict"], &["strict"])?;
    parsed.positionals(1, "check <perfiles.json> [--strict]")?;
    let set = load_set(&parsed.positionals[0])?;
    if set.is_empty() {
        return Err(runtime("el conjunto no contiene campeones"));
    }
    let cells = set_cell_counts(&set);
    let provenance = set_provenance_counts(&set);
    let roles = role_count(&set);
    let roles_inferred = set
        .champions
        .values()
        .flat_map(|profile| profile.roles.iter())
        .filter(|role| matches!(role.source, Provenance::Inferred { .. }))
        .count();

    println!("{} v{}", set.id, set.version);
    if let Some(patch) = &set.patch {
        println!("  parche: {patch}");
    }
    println!("  taxonomía: {}", set.taxonomy_version);
    println!("  campeones: {}", set.len());
    println!("  roles: {} ({} inferidos)", roles, roles_inferred);
    println!(
        "  celdas: {} ({} conocidas, {} desconocidas)",
        cells.total(),
        cells.known(),
        cells.unknown
    );
    println!(
        "  procedencia: riot {} · curado {} · importado {} · inferido {} · desconocido {}",
        provenance.riot,
        provenance.curated,
        provenance.imported,
        provenance.inferred,
        provenance.unknown
    );
    println!("  cobertura media: {}", percent(average_coverage(&set)));

    if provenance.inferred > 0 {
        println!(
            "Aviso: {} celdas siguen siendo inferidas; el conjunto no está curado.",
            provenance.inferred
        );
    }
    if provenance.unknown > 0 {
        println!(
            "Aviso: {} celdas son desconocidas; la UI las mostrará como tales, no como cero.",
            provenance.unknown
        );
    }
    if parsed.has("strict") && provenance.inferred > 0 {
        return Err(runtime(format!(
            "check estricto: quedan {} celdas inferidas",
            provenance.inferred
        )));
    }
    println!("Conjunto válido.");
    Ok(())
}

fn read_text(path: &str) -> Result<String, CliError> {
    std::fs::read_to_string(path)
        .map_err(|error| runtime(format!("no se pudo leer {path}: {error}")))
}

fn load_set(path: &str) -> Result<ProfileSet, CliError> {
    let text = read_text(path)?;
    ProfileSet::from_json(&text, taxonomy()).map_err(|error| runtime(format!("{path}: {error}")))
}

fn write_set(path: &str, set: &ProfileSet) -> Result<(), CliError> {
    set.validate(taxonomy())
        .map_err(|error| runtime(format!("el conjunto resultante no es válido: {error}")))?;
    let json = set
        .to_json()
        .map_err(|error| runtime(format!("no se pudo serializar el conjunto: {error}")))?;
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|error| {
                runtime(format!(
                    "no se pudo crear el directorio {}: {error}",
                    parent.display()
                ))
            })?;
        }
    }
    std::fs::write(path, json)
        .map_err(|error| runtime(format!("no se pudo escribir {path}: {error}")))
}

fn find_profile<'a>(set: &'a ProfileSet, needle: &str) -> Result<&'a ChampionProfile, CliError> {
    let needle = needle.trim();
    if let Ok(key) = needle.parse::<u16>() {
        let key = ChampionKey(key);
        return set
            .get(key)
            .ok_or_else(|| runtime(format!("el campeón {key} no está en el conjunto")));
    }
    let lower = needle.to_lowercase();
    set.champions
        .values()
        .find(|profile| {
            profile.slug.to_lowercase() == lower || profile.name.to_lowercase() == lower
        })
        .ok_or_else(|| runtime(format!("no hay un campeón `{needle}` en el conjunto")))
}

fn parse_champion_keys(values: &[String]) -> Result<Vec<ChampionKey>, CliError> {
    let mut keys = Vec::new();
    for value in values {
        for part in value.split(',') {
            let part = part.trim();
            if part.is_empty() {
                return Err(usage("`--champion` contiene una clave vacía"));
            }
            let key = part
                .parse::<u16>()
                .map_err(|_| usage(format!("`--champion` debe ser una clave numérica: {part}")))?;
            keys.push(ChampionKey(key));
        }
    }
    Ok(keys)
}

fn role_count(set: &ProfileSet) -> usize {
    set.champions
        .values()
        .map(|profile| profile.roles.len())
        .sum()
}

fn average_coverage(set: &ProfileSet) -> f32 {
    if set.is_empty() {
        return 0.0;
    }
    let total: f32 = set
        .champions
        .values()
        .map(ChampionProfile::known_cell_ratio)
        .sum();
    total / set.len() as f32
}

fn print_identity(set: &ProfileSet) {
    println!("  id: {}", set.id);
    println!("  versión: {}", set.version);
    if let Some(patch) = &set.patch {
        println!("  parche: {patch}");
    }
}

fn print_report(report: &CurateReport, mode: CurationMode) {
    let verb = match mode {
        CurationMode::Review => "convertidas",
        CurationMode::Approve => "aprobadas",
    };
    println!("  campeones incluidos: {}", report.champions);
    println!("  roles {verb}: {}", report.roles);
    println!("  arquetipos {verb}: {}", report.archetypes);
    println!("  traits {verb}: {}", report.traits);
    if report.cells() == 0 {
        println!("  no había celdas inferidas conocidas que convertir");
    }
    if report.skipped > 0 {
        println!(
            "  celdas presentes ya con fuente o desconocidas, sin tocar: {}",
            report.skipped
        );
    }
}

fn describe_source(source: &Provenance, confidence: f32) -> String {
    let detail = match source {
        Provenance::Riot => "Riot".to_string(),
        Provenance::Curated { editor, review } => {
            format!("curado por {editor} · revisado por {review}")
        }
        Provenance::Imported { source, dataset } => {
            format!("importado de {source} ({dataset})")
        }
        Provenance::Inferred { method, version } => {
            format!("inferido · {method} v{version}")
        }
    };
    format!("{detail} · confianza {}", percent(confidence))
}

fn percent(value: f32) -> String {
    format!("{:.0} %", value * 100.0)
}

fn list_or_none(values: &[&str]) -> String {
    if values.is_empty() {
        "—".to_string()
    } else {
        values.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn strict_is_a_switch_and_does_not_consume_a_positional() {
        let parsed = parse_args(&args(&["--strict", "set.json"]), &["strict"], &["strict"])
            .expect("argumentos válidos");
        assert!(parsed.has("strict"));
        assert_eq!(parsed.positionals, vec!["set.json".to_string()]);
    }

    #[test]
    fn duplicate_single_value_options_are_rejected() {
        let parsed = parse_args(
            &args(&["--out", "a.json", "--out", "b.json"]),
            &["out"],
            &[],
        )
        .expect("se parsea, el error llega al usar la opción");
        assert!(matches!(parsed.single("out"), Err(CliError::Usage(_))));
    }

    #[test]
    fn champion_keys_accept_commas_and_repeats() {
        let keys = parse_champion_keys(&args(&["111, 266", "150"])).expect("claves válidas");
        assert_eq!(
            keys,
            vec![ChampionKey(111), ChampionKey(266), ChampionKey(150)]
        );
    }

    #[test]
    fn unknown_options_and_invalid_keys_are_rejected() {
        assert!(matches!(
            parse_args(&args(&["--nope"]), &[], &[]),
            Err(CliError::Usage(_))
        ));
        assert!(matches!(
            parse_champion_keys(&args(&["Nautilus"])),
            Err(CliError::Usage(_))
        ));
    }
}
