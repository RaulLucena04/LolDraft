# 09 — Curación de perfiles

La curación convierte propuestas automáticas (`Inferred`) en datos revisados (`Curated`) sin
inventar celdas. La herramienta vive fuera de la app para que el motor siga siendo puro y para que
cada publicación pueda revisarse antes de entrar en SQLite.

## Reglas de honestidad

- `review` y `approve` solo convierten celdas **conocidas** que ya existen y tienen procedencia
  `Inferred`. Las celdas `Riot`, `Curated` e `Imported` no se tocan.
- Una celda desconocida (`None`) sigue desconocida después de aprobar el conjunto. No se rellena con
  ceros, valores neutros ni heurísticas.
- `edit` es la única vía para dar valor a una celda desconocida, y solo porque **una persona aporta
  ese valor explícitamente**. La celda queda como `Curated` con editor y revisor.
- `Curated` guarda editor y revisor. Si coinciden, la CLI lo avisa: la procedencia documenta una
  segunda revisión.
- Los tres shares de daño deben sumar 1 ± 0.01. Si se edita uno, hay que editar los tres en la misma
  operación o dejar el resto desconocido; la CLI valida antes de escribir.
- La app nunca presenta un conjunto inferido como curado. El drawer y el panel de análisis muestran
  la procedencia de cada celda.

## Flujo recomendado

Desde la raíz del repositorio:

```powershell
# 1. Expandir la propuesta del piloto a un conjunto de perfiles Inferred.
cargo run -p loldraft-curation --bin loldraft-curate -- `
  from-proposals crates/loldraft-engine/data/profiles-pilot-v1.proposals.json `
  --out work/pilot-inferred.json

# 2. Revisar cobertura y detalle antes de tocar nada.
cargo run -p loldraft-curation --bin loldraft-curate -- list work/pilot-inferred.json
cargo run -p loldraft-curation --bin loldraft-curate -- show work/pilot-inferred.json Nautilus

# 3. Generar un parte de revisión para el subconjunto elegido.
cargo run -p loldraft-curation --bin loldraft-curate -- `
  worksheet work/pilot-inferred.json --champion 111,266 --out work/parte.md

# 4a. Aprobar celdas inferidas ya conocidas con una identidad nueva.
cargo run -p loldraft-curation --bin loldraft-curate -- `
  review work/pilot-inferred.json `
  --champion 111 --editor "tu nombre" --reviewer "segunda revisión" `
  --confidence 0.8 --id pilot-2026-11-reviewed --version 2026.11.2 `
  --out work/pilot-reviewed.json

# 4b. O ajustar celdas concretas con una decisión humana explícita.
#     `unknown` borra el valor; un número en 0..=1 lo fija.
cargo run -p loldraft-curation --bin loldraft-curate -- `
  edit work/pilot-inferred.json --champion 111 `
  --set engage=0.8 --set poke=0.2 --set mobility=unknown `
  --role support=0.9 `
  --editor "tu nombre" --reviewer "segunda revisión" --confidence 0.85 `
  --id pilot-2026-11-reviewed --version 2026.11.2 `
  --out work/pilot-reviewed.json

# 5. Auditar los cambios respecto al conjunto original antes de importar.
cargo run -p loldraft-curation --bin loldraft-curate -- `
  diff work/pilot-inferred.json work/pilot-reviewed.json

# 6. Comprobar el resultado. `--strict` falla si aún quedan celdas inferidas.
cargo run -p loldraft-curation --bin loldraft-curate -- check work/pilot-reviewed.json --strict
```

El parte de revisión es Markdown y no modifica datos: sirve para anotar `ok`, `ajustar <valor>` o
`desconocido` y luego aplicar los cambios con `edit`. El fichero resultante se importa desde la
pantalla de inicio con **Importar conjunto revisado**. La importación es transaccional y activa el
conjunto solo si se pide.

## Comandos

| Comando | Qué hace |
| --- | --- |
| `from-proposals <propuestas.json> --out <salida.json>` | Expande una propuesta a un conjunto validado. Todo lo declarado queda `Inferred`. |
| `list <perfiles.json>` | Lista campeones, roles y celdas conocidas/desconocidas. |
| `show <perfiles.json> <clave\|slug\|nombre>` | Muestra roles, daño, arquetipos, traits y celdas desconocidas de un campeón. |
| `worksheet <perfiles.json> [--out <salida.md>] [--champion <clave>...]` | Genera un parte de revisión legible con roles, arquetipos, traits, valores y procedencia. Sin `--out`, imprime por stdout. |
| `edit <perfiles.json> --champion <clave> --set <celda>=<valor> ... --editor <n> --reviewer <n> --confidence <0..1> --out <salida.json>` | Edita celdas y roles con una decisión humana explícita. Las celdas tocadas pasan a `Curated`. |
| `review <perfiles.json> --editor <n> --reviewer <n> --confidence <0..1> --out <salida.json>` | Convierte celdas `Inferred` conocidas en `Curated` y fija su confianza. |
| `approve <perfiles.json> --editor <n> --reviewer <n> --out <salida.json>` | Igual que `review`, pero conserva la confianza existente. |
| `check <perfiles.json> [--strict]` | Valida el conjunto y resume procedencia y cobertura. |
| `diff <antes.json> <despues.json> [--champion <clave>...]` | Compara dos conjuntos: cambios de valor, procedencia y confianza, celdas rellenadas o vaciadas y campeones añadidos o eliminados. No escribe nada. |

Opciones de `review` y `approve`:

- `--champion <clave>`: limita la operación a esos campeones. Se puede repetir o separar por comas.
- `--id <id>` y `--version <versión>`: nueva identidad de la publicación. Si el id conserva
  `inferred`, la CLI avisa.
- `--out <salida.json>`: obligatorio.

Opciones de `edit`:

- `--champion <clave>`: un solo campeón por ejecución, para que cada cambio tenga un responsable claro.
- `--set <celda>=<valor>`: arquetipo o trait por su id (`engage`, `damage_physical`, …). Repetible.
  El valor va de 0 a 1 o es `unknown`.
- `--role <rol>=<valor>`: proficiencia de un rol ya declarado. Repetible.
- `--editor`, `--reviewer`, `--confidence`, `--out`: obligatorios, como en `review`.
- `--id` y `--version`: opcionales, igual que en `review`.

Opciones de `diff`:

- `--champion <clave>`: limita la comparación a esos campeones. Se puede repetir o separar por
  comas. Una clave que no exista en ninguno de los dos conjuntos es un error, no una comparación
  vacía.

`diff` agrupa las celdas cuyo valor no cambió pero sí su procedencia o confianza, para que una
revisión completa no oculte los cambios reales entre cientos de conversiones `Inferred` → `Curated`.
Los cambios de valor, las celdas rellenadas o vaciadas y los roles se listan uno a uno.

## Qué no hace

- No propone valores por sí sola: `edit` exige que una persona indique el valor y la confianza.
- No importa un parte de revisión rellenado: hay que aplicar las decisiones con `edit`.
- No genera propuestas nuevas ni analiza kits.
- `diff` no escribe ficheros ni modifica conjuntos: solo compara y muestra.
- No escribe en SQLite: la importación pasa por la app y por `profiles_import_set`.
- No convierte un conjunto incompleto en completo: la cobertura desconocida se mantiene visible.
