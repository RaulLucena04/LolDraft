//! `loldraft-curate`: herramienta de curación de perfiles por fichero.
//!
//! No abre la base de datos ni habla con la app. Lee y escribe conjuntos de
//! perfiles validados para que una revisión pueda inspeccionarse antes de
//! importarla.

use loldraft_curation::{
    apply_identity, apply_profile_edits, curate_inferred_cells, diff_profile_sets,
    profile_cell_counts, profile_provenance_counts, same_curator, set_cell_counts,
    set_provenance_counts, CellChange, CellEdit, CellTarget, CurateReport, ReviewSelection,
    RoleEdit, RoleState, SetDiff,
};
use loldraft_domain::{ChampionKey, Role};
use loldraft_engine::{
    taxonomy, ArchetypeId, ChampionProfile, ProfileProposalSet, ProfileSet, Provenance, TraitId,
};
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
  worksheet <perfiles.json> [--out <salida.md>] [--champion <clave>...]
      Genera un parte de revisión legible con todos los roles, arquetipos y
      traits, sus valores actuales y una columna para anotar la revisión.
  edit <perfiles.json> --champion <clave> --set <celda>=<valor> ... --editor <n> --reviewer <n> --confidence <0..1> --out <salida.json>
      Edita explícitamente celdas y roles con decisión humana. Las celdas
      tocadas pasan a `Curated`. Puede rellenar una celda desconocida o
      vaciarla con `--set <celda>=unknown`.
  review <perfiles.json> --editor <n> --reviewer <n> --confidence <0..1> --out <salida.json>
      Convierte celdas `Inferred` conocidas en `Curated` y fija su confianza.
  approve <perfiles.json> --editor <n> --reviewer <n> --out <salida.json>
      Convierte celdas `Inferred` conocidas en `Curated` conservando su
      confianza. No rellena celdas desconocidas.
  check <perfiles.json> [--strict]
      Valida el conjunto y resume su cobertura y procedencia.
  diff <antes.json> <despues.json> [--champion <clave>...]
      Compara dos conjuntos y lista cambios de valor, procedencia y
      confianza, además de campeones añadidos o eliminados.

OPCIONES DE REVIEW/APPROVE
  --champion <clave>   Limita la revisión a esos campeones. Repetible o con
                       varias claves separadas por comas.
  --id <id>            Nuevo id del conjunto publicado.
  --version <versión>  Nueva versión del conjunto publicado.
  --out <salida.json>  Fichero de salida (obligatorio).

OPCIONES DE DIFF
  --champion <clave>   Limita la comparación a esos campeones. Repetible o
                       con varias claves separadas por comas.

OPCIONES DE EDIT
  --set <celda>=<valor>  Arquetipo o trait por su id (p. ej. engage=0.8).
                         Repetible. Valor en 0..=1 o `unknown`.
  --role <rol>=<valor>   Proficiencia de un rol ya declarado. Repetible.
  --editor, --reviewer, --confidence, --out  Obligatorios, como en `review`.

OTRAS OPCIONES
  --strict             `check` falla si quedan celdas inferidas.
  -h, --help           Muestra esta ayuda.
  -V, --version        Muestra la versión.

La herramienta nunca inventa celdas por su cuenta: `review`/`approve` solo
cambian la procedencia de celdas que ya existen. `edit` es una decisión humana
explícita y queda registrada como `Curated` con editor y revisor. Las celdas
desconocidas se muestran como desconocidas; no se asumen cero. Si el editor y
el revisor son la misma persona, se avisa: `Curated` documenta una segunda
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
        "worksheet" => worksheet(&args[1..]),
        "edit" => edit(&args[1..]),
        "review" => curate(&args[1..], CurationMode::Review),
        "approve" => curate(&args[1..], CurationMode::Approve),
        "check" => check(&args[1..]),
        "diff" => diff(&args[1..]),
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

fn worksheet(args: &[String]) -> Result<(), CliError> {
    let parsed = parse_args(args, &["out", "champion"], &[])?;
    parsed.positionals(
        1,
        "worksheet <perfiles.json> [--out <salida.md>] [--champion <clave>...]",
    )?;
    let set = load_set(&parsed.positionals[0])?;
    if set.is_empty() {
        return Err(runtime("el conjunto no contiene campeones"));
    }
    let selection = ReviewSelection::only(parse_champion_keys(parsed.many("champion"))?);
    let keys = selection
        .resolve(&set)
        .map_err(|error| runtime(error.to_string()))?;
    let champion_count = keys.len();

    let mut out = String::new();
    writeln!(out, "# Parte de revisión — {} v{}", set.id, set.version)
        .expect("escritura en memoria");
    match &set.patch {
        Some(patch) => writeln!(
            out,
            "\nParche {patch} · taxonomía {} · {champion_count} campeones",
            set.taxonomy_version
        ),
        None => writeln!(
            out,
            "\nTaxonomía {} · {champion_count} campeones",
            set.taxonomy_version
        ),
    }
    .expect("escritura en memoria");
    out.push_str(
        "\nEste parte no modifica datos. Anota cada celda en la columna **Revisión** \
         (`ok`, `ajustar <valor>` o `desconocido`) y aplica los cambios con \
         `loldraft-curate edit`.\n",
    );
    out.push_str(
        "\nAnclas: 0 No aporta · 0.25 Incidental · 0.5 Sólida · 0.75 Fuerte · 1 Definitoria\n",
    );
    out.push_str(
        "\nLos tres shares de daño (`damage_physical`, `damage_magic`, `damage_true`) \
         deben sumar 100 %.\n",
    );

    for key in keys {
        if let Some(profile) = set.get(key) {
            render_worksheet_profile(&mut out, profile);
        }
    }

    match parsed.single("out")? {
        Some(path) => {
            write_text(path, &out)?;
            println!("Parte de revisión escrito en {path}");
            println!("  campeones: {champion_count}");
        }
        None => print!("{out}"),
    }
    Ok(())
}

fn render_worksheet_profile(out: &mut String, profile: &ChampionProfile) {
    writeln!(
        out,
        "\n## {} ({}) · {}\n",
        profile.name, profile.key, profile.slug
    )
    .expect("escritura en memoria");

    let counts = profile_cell_counts(profile);
    writeln!(
        out,
        "Celdas: {} conocidas de {} · {} observadas · {} inferidas · {} desconocidas\n",
        counts.known(),
        counts.total(),
        counts.observed,
        counts.inferred,
        counts.unknown
    )
    .expect("escritura en memoria");

    out.push_str("### Roles\n\n");
    out.push_str("| Rol | id | Proficiencia | Procedencia | Revisión |\n");
    out.push_str("| --- | --- | --- | --- | --- |\n");
    for role in &profile.roles {
        writeln!(
            out,
            "| {} | `{}` | {} | {} |  |",
            escape_md(role.role.label_es()),
            role.role.as_str(),
            percent(role.proficiency),
            escape_md(&worksheet_source(&role.source, role.confidence))
        )
        .expect("escritura en memoria");
    }

    out.push_str("\n### Arquetipos\n\n");
    out.push_str("| Celda | id | Valor | Ancla | Procedencia | Revisión |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- |\n");
    let mut archetypes: Vec<_> = ArchetypeId::ALL
        .iter()
        .map(|id| {
            let cell = profile.archetypes.get(id).and_then(|rating| {
                rating
                    .intensity
                    .map(|value| (value, &rating.source, rating.confidence))
            });
            (*id, cell)
        })
        .collect();
    archetypes.sort_by(|left, right| {
        sort_known_first(
            left.1.map(|(value, _, _)| value),
            right.1.map(|(value, _, _)| value),
            left.0.as_str(),
            right.0.as_str(),
        )
    });
    for (id, cell) in archetypes {
        let label = taxonomy()
            .archetype(id)
            .map(|entry| entry.label_es.as_str())
            .unwrap_or_else(|| id.as_str());
        let (value, anchor, source) = match cell {
            Some((value, source, confidence)) => (
                percent(value),
                taxonomy()
                    .intensity_label(Some(value))
                    .unwrap_or("—")
                    .to_string(),
                worksheet_source(source, confidence),
            ),
            None => ("—".to_string(), "—".to_string(), "desconocido".to_string()),
        };
        writeln!(
            out,
            "| {} | `{}` | {} | {} | {} |  |",
            escape_md(label),
            id.as_str(),
            value,
            escape_md(&anchor),
            escape_md(&source)
        )
        .expect("escritura en memoria");
    }

    out.push_str("\n### Traits\n\n");
    out.push_str("| Celda | id | Valor | Procedencia | Revisión |\n");
    out.push_str("| --- | --- | --- | --- | --- |\n");
    let mut traits: Vec<_> = TraitId::ALL
        .iter()
        .map(|id| {
            let cell = profile.traits.get(id).and_then(|rating| {
                rating
                    .value
                    .map(|value| (value, &rating.source, rating.confidence))
            });
            (*id, cell)
        })
        .collect();
    traits.sort_by(|left, right| {
        sort_known_first(
            left.1.map(|(value, _, _)| value),
            right.1.map(|(value, _, _)| value),
            left.0.as_str(),
            right.0.as_str(),
        )
    });
    for (id, cell) in traits {
        let label = taxonomy()
            .trait_entry(id)
            .map(|entry| entry.label_es.as_str())
            .unwrap_or_else(|| id.as_str());
        let (value, source) = match cell {
            Some((value, source, confidence)) => {
                (percent(value), worksheet_source(source, confidence))
            }
            None => ("—".to_string(), "desconocido".to_string()),
        };
        writeln!(
            out,
            "| {} | `{}` | {} | {} |  |",
            escape_md(label),
            id.as_str(),
            value,
            escape_md(&source)
        )
        .expect("escritura en memoria");
    }
}

fn sort_known_first(
    left: Option<f32>,
    right: Option<f32>,
    left_id: &str,
    right_id: &str,
) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.total_cmp(&left).then_with(|| left_id.cmp(right_id)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left_id.cmp(right_id),
    }
}

fn edit(args: &[String]) -> Result<(), CliError> {
    let parsed = parse_args(
        args,
        &[
            "champion",
            "set",
            "role",
            "editor",
            "reviewer",
            "confidence",
            "id",
            "version",
            "out",
        ],
        &[],
    )?;
    parsed.positionals(
        1,
        "edit <perfiles.json> --champion <clave> --set <celda>=<valor> ... --editor <n> --reviewer <n> --confidence <0..1> --out <salida.json>",
    )?;
    let input = &parsed.positionals[0];
    let out = parsed.required("out")?;
    let editor = parsed.required("editor")?;
    let reviewer = parsed.required("reviewer")?;
    let confidence = parse_confidence(parsed.required("confidence")?)?;
    let champion = parse_single_champion_key(parsed.required("champion")?)?;
    let cells = parse_cell_edits(parsed.many("set"))?;
    let roles = parse_role_edits(parsed.many("role"))?;
    if cells.is_empty() && roles.is_empty() {
        return Err(usage("no hay nada que editar: usa `--set` o `--role`"));
    }

    let mut set = load_set(input)?;
    apply_identity(&mut set, parsed.single("id")?, parsed.single("version")?)
        .map_err(|error| usage(error.to_string()))?;
    let report = apply_profile_edits(
        &mut set, champion, &cells, &roles, editor, reviewer, confidence,
    )
    .map_err(|error| runtime(error.to_string()))?;
    write_set(out, &set)?;

    println!("Perfil editado y escrito en {out}");
    print_identity(&set);
    println!("  celdas editadas: {}", report.cells);
    println!("  roles editados: {}", report.roles);
    if report.filled_unknown > 0 {
        println!(
            "  celdas desconocidas rellenadas con decisión humana: {}",
            report.filled_unknown
        );
    }
    if same_curator(editor, reviewer) {
        println!(
            "Aviso: el editor y el revisor coinciden; `Curated` documenta una revisión por una segunda persona."
        );
    }
    Ok(())
}

fn parse_confidence(raw: &str) -> Result<f32, CliError> {
    let value = raw
        .parse::<f32>()
        .map_err(|_| usage(format!("`--confidence` no es un número: {raw}")))?;
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(usage(format!(
            "`--confidence` debe estar en 0..=1; recibido {raw}"
        )));
    }
    Ok(value)
}

fn parse_single_champion_key(raw: &str) -> Result<ChampionKey, CliError> {
    let trimmed = raw.trim();
    if trimmed.contains(',') {
        return Err(usage(
            "`--champion` en `edit` admite una sola clave; ejecuta el comando una vez por campeón",
        ));
    }
    let key = trimmed.parse::<u16>().map_err(|_| {
        usage(format!(
            "`--champion` debe ser una clave numérica: {trimmed}"
        ))
    })?;
    Ok(ChampionKey(key))
}

fn parse_cell_edits(values: &[String]) -> Result<Vec<CellEdit>, CliError> {
    let mut edits = Vec::new();
    for raw in values {
        let (code, raw_value) = raw.split_once('=').ok_or_else(|| {
            usage(format!(
                "`--set` espera `<celda>=<valor>` (p. ej. engage=0.8): {raw}"
            ))
        })?;
        let code = code.trim();
        if code.is_empty() {
            return Err(usage("`--set` contiene una celda vacía"));
        }
        edits.push(CellEdit {
            target: resolve_cell(code)?,
            value: parse_cell_value(raw_value.trim())?,
        });
    }
    Ok(edits)
}

fn resolve_cell(code: &str) -> Result<CellTarget, CliError> {
    let archetype = ArchetypeId::from_code(code);
    let trait_id = TraitId::from_code(code);
    match (archetype, trait_id) {
        (Some(id), None) => Ok(CellTarget::Archetype(id)),
        (None, Some(id)) => Ok(CellTarget::Trait(id)),
        (Some(_), Some(_)) => Err(usage(format!(
            "la celda `{code}` es ambigua entre arquetipo y trait"
        ))),
        (None, None) => Err(usage(format!(
            "no existe la celda `{code}`. Usa `worksheet` para ver los ids."
        ))),
    }
}

fn parse_cell_value(raw: &str) -> Result<Option<f32>, CliError> {
    if raw.eq_ignore_ascii_case("unknown") || raw.eq_ignore_ascii_case("desconocido") {
        return Ok(None);
    }
    let value = raw
        .parse::<f32>()
        .map_err(|_| usage(format!("valor de celda no válido: {raw}")))?;
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(usage(format!(
            "el valor debe estar en 0..=1 o ser `unknown`: {raw}"
        )));
    }
    Ok(Some(value))
}

fn parse_role_edits(values: &[String]) -> Result<Vec<RoleEdit>, CliError> {
    let mut edits = Vec::new();
    for raw in values {
        let (code, raw_value) = raw.split_once('=').ok_or_else(|| {
            usage(format!(
                "`--role` espera `<rol>=<valor>` (p. ej. support=0.9): {raw}"
            ))
        })?;
        let code = code.trim();
        let role = Role::from_code(code).ok_or_else(|| {
            usage(format!(
                "rol desconocido `{code}`; usa top, jungle, mid, adc o support"
            ))
        })?;
        let value = raw_value
            .trim()
            .parse::<f32>()
            .map_err(|_| usage(format!("proficiencia no válida: {raw_value}")))?;
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(usage(format!(
                "la proficiencia debe estar en 0..=1; recibido {raw_value}"
            )));
        }
        edits.push(RoleEdit {
            role,
            proficiency: value,
        });
    }
    Ok(edits)
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
        CurationMode::Review => Some(parse_confidence(parsed.required("confidence")?)?),
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

fn diff(args: &[String]) -> Result<(), CliError> {
    let parsed = parse_args(args, &["champion"], &[])?;
    parsed.positionals(
        2,
        "diff <antes.json> <despues.json> [--champion <clave>...]",
    )?;
    let before = load_set(&parsed.positionals[0])?;
    let after = load_set(&parsed.positionals[1])?;
    let selection = ReviewSelection::only(parse_champion_keys(parsed.many("champion"))?);
    if !selection.is_all() {
        for key in selection.keys() {
            if before.get(key).is_none() && after.get(key).is_none() {
                return Err(runtime(format!(
                    "el campeón {key} no está en ninguno de los dos conjuntos"
                )));
            }
        }
    }

    let changes = diff_profile_sets(&before, &after, &selection);
    println!(
        "Diferencias: {} v{} → {} v{}",
        before.id, before.version, after.id, after.version
    );
    if !selection.is_all() {
        let keys = selection
            .keys()
            .map(|key| key.to_string())
            .collect::<Vec<_>>();
        println!("  selección: {}", keys.join(", "));
    }
    if changes.is_empty() {
        println!("Sin diferencias en la selección.");
        return Ok(());
    }

    print_diff_summary(&changes);
    print!("{}", render_diff(&changes));
    Ok(())
}

fn print_diff_summary(changes: &SetDiff) {
    let cells = changes.cells();
    let value_cells = changes
        .champions
        .iter()
        .flat_map(|champion| &champion.cells)
        .filter(|cell| cell.before != cell.after)
        .count();
    println!("  campeones con cambios: {}", changes.champions.len());
    println!(
        "  celdas: {cells} (valor {value_cells} · solo procedencia/confianza {})",
        cells - value_cells
    );
    println!(
        "  rellenadas: {} · vaciadas: {}",
        changes.filled(),
        changes.cleared()
    );
    println!("  roles: {}", changes.roles());
    println!(
        "  campeones añadidos: {} · eliminados: {}",
        changes.added.len(),
        changes.removed.len()
    );
}

fn render_diff(changes: &SetDiff) -> String {
    let mut out = String::new();
    for champion in &changes.champions {
        writeln!(out, "\n## {} ({})", champion.name, champion.champion)
            .expect("escritura en memoria");
        if !champion.roles.is_empty() {
            out.push_str("  roles\n");
            for role in &champion.roles {
                writeln!(
                    out,
                    "    {}: {} → {}",
                    role.role.label_es(),
                    role_state_text(role.before.as_ref()),
                    role_state_text(role.after.as_ref())
                )
                .expect("escritura en memoria");
            }
        }

        let mut value_changes: Vec<&CellChange> = Vec::new();
        let mut transitions: BTreeMap<String, usize> = BTreeMap::new();
        for cell in &champion.cells {
            if cell.before != cell.after {
                value_changes.push(cell);
            } else {
                *transitions.entry(provenance_transition(cell)).or_default() += 1;
            }
        }
        if !value_changes.is_empty() {
            out.push_str("  celdas\n");
            for cell in value_changes {
                writeln!(
                    out,
                    "    {} (`{}`): {} → {}",
                    cell.target.label_es(),
                    cell.target.code(),
                    cell_state_text(
                        cell.before,
                        cell.before_source.as_ref(),
                        cell.before_confidence
                    ),
                    cell_state_text(
                        cell.after,
                        cell.after_source.as_ref(),
                        cell.after_confidence
                    )
                )
                .expect("escritura en memoria");
            }
        }
        if !transitions.is_empty() {
            out.push_str("  procedencia/confianza\n");
            for (transition, count) in transitions {
                writeln!(out, "    {transition} · {count} celdas").expect("escritura en memoria");
            }
        }
    }

    if !changes.added.is_empty() {
        out.push_str("\nAñadidos\n");
        for (key, name) in &changes.added {
            writeln!(out, "  {:>4}  {}", key.0, name).expect("escritura en memoria");
        }
    }
    if !changes.removed.is_empty() {
        out.push_str("\nEliminados\n");
        for (key, name) in &changes.removed {
            writeln!(out, "  {:>4}  {}", key.0, name).expect("escritura en memoria");
        }
    }
    out
}

fn cell_state_text(value: Option<f32>, source: Option<&Provenance>, confidence: f32) -> String {
    match (value, source) {
        (Some(value), Some(source)) => {
            format!(
                "{} · {}",
                percent(value),
                worksheet_source(source, confidence)
            )
        }
        (Some(value), None) => percent(value),
        (None, _) => "desconocido".to_string(),
    }
}

fn role_state_text(state: Option<&RoleState>) -> String {
    match state {
        Some(state) => format!(
            "{} · {}",
            percent(state.proficiency),
            worksheet_source(&state.source, state.confidence)
        ),
        None => "sin rol".to_string(),
    }
}

fn provenance_transition(cell: &CellChange) -> String {
    let before = match cell.before_source.as_ref() {
        Some(source) => format!(
            "{} {}",
            source_kind(source),
            percent(cell.before_confidence)
        ),
        None => "desconocido".to_string(),
    };
    let after = match cell.after_source.as_ref() {
        Some(source) => format!("{} {}", source_kind(source), percent(cell.after_confidence)),
        None => "desconocido".to_string(),
    };
    format!("{before} → {after}")
}

fn source_kind(source: &Provenance) -> &'static str {
    match source {
        Provenance::Riot => "Riot",
        Provenance::Curated { .. } => "curado",
        Provenance::Imported { .. } => "importado",
        Provenance::Inferred { .. } => "inferido",
    }
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
    write_text(path, &json)
}

fn write_text(path: &str, contents: &str) -> Result<(), CliError> {
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
    std::fs::write(path, contents)
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

fn worksheet_source(source: &Provenance, confidence: f32) -> String {
    match source {
        Provenance::Riot => "Riot".to_string(),
        Provenance::Curated { editor, review } => {
            format!("curado · {editor} / {review} · {}", percent(confidence))
        }
        Provenance::Imported { source, dataset } => format!("importado · {source} ({dataset})"),
        Provenance::Inferred { method, version } => {
            format!("inferido · {method} v{version} · {}", percent(confidence))
        }
    }
}

fn escape_md(value: &str) -> String {
    value.replace('|', "\\|")
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

    #[test]
    fn cell_edits_parse_values_and_unknown() {
        let edits = parse_cell_edits(&args(&["engage=0.8", "mobility=unknown"]))
            .expect("ediciones válidas");
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].target, CellTarget::Archetype(ArchetypeId::Engage));
        assert_eq!(edits[0].value, Some(0.8));
        assert_eq!(edits[1].target, CellTarget::Trait(TraitId::Mobility));
        assert_eq!(edits[1].value, None);
    }

    #[test]
    fn cell_edits_reject_unknown_ids_bad_values_and_missing_equals() {
        assert!(matches!(
            parse_cell_edits(&args(&["nope=0.5"])),
            Err(CliError::Usage(_))
        ));
        assert!(matches!(
            parse_cell_edits(&args(&["engage=2"])),
            Err(CliError::Usage(_))
        ));
        assert!(matches!(
            parse_cell_edits(&args(&["engage"])),
            Err(CliError::Usage(_))
        ));
    }

    #[test]
    fn role_edits_parse_known_roles_only() {
        let edits = parse_role_edits(&args(&["support=0.9"])).expect("rol válido");
        assert_eq!(edits[0].role, Role::Support);
        assert!(matches!(
            parse_role_edits(&args(&["adc=1.5"])),
            Err(CliError::Usage(_))
        ));
        assert!(matches!(
            parse_role_edits(&args(&["carry=0.5"])),
            Err(CliError::Usage(_))
        ));
    }
}
