# 09 — Curación de perfiles

La curación convierte propuestas automáticas (`Inferred`) en datos revisados (`Curated`) sin
inventar celdas. La herramienta vive fuera de la app para que el motor siga siendo puro y para que
cada publicación pueda revisarse antes de entrar en SQLite.

## Reglas de honestidad

- Solo se convierten celdas **conocidas** que ya existen y tienen procedencia `Inferred`.
- Las celdas `Riot`, `Curated` e `Imported` no se tocan.
- Una celda desconocida (`None`) sigue desconocida después de aprobar el conjunto. No se rellena con
  ceros ni con valores neutros.
- `Curated` guarda editor y revisor. Si coinciden, la CLI lo avisa: la procedencia documenta una
  segunda revisión.
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

# 3. Revisar un subconjunto con la confianza del revisor y una identidad nueva.
cargo run -p loldraft-curation --bin loldraft-curate -- `
  review work/pilot-inferred.json `
  --champion 111 --editor "tu nombre" --reviewer "segunda revisión" `
  --confidence 0.8 --id pilot-2026-11-reviewed --version 2026.11.2 `
  --out work/pilot-reviewed.json

# 4. Comprobar el resultado. `--strict` falla si aún quedan celdas inferidas.
cargo run -p loldraft-curation --bin loldraft-curate -- check work/pilot-reviewed.json --strict
```

El fichero resultante se importa desde la pantalla de inicio con **Importar conjunto revisado**.
La importación es transaccional y activa el conjunto solo si se pide.

## Comandos

| Comando | Qué hace |
| --- | --- |
| `from-proposals <propuestas.json> --out <salida.json>` | Expande una propuesta a un conjunto validado. Todo lo declarado queda `Inferred`. |
| `list <perfiles.json>` | Lista campeones, roles y celdas conocidas/desconocidas. |
| `show <perfiles.json> <clave\|slug\|nombre>` | Muestra roles, daño, arquetipos, traits y celdas desconocidas de un campeón. |
| `review <perfiles.json> --editor <n> --reviewer <n> --confidence <0..1> --out <salida.json>` | Convierte celdas `Inferred` conocidas en `Curated` y fija su confianza. |
| `approve <perfiles.json> --editor <n> --reviewer <n> --out <salida.json>` | Igual que `review`, pero conserva la confianza existente. |
| `check <perfiles.json> [--strict]` | Valida el conjunto y resume procedencia y cobertura. |

Opciones de `review` y `approve`:

- `--champion <clave>`: limita la operación a esos campeones. Se puede repetir o separar por comas.
- `--id <id>` y `--version <versión>`: nueva identidad de la publicación. Si el id conserva
  `inferred`, la CLI avisa.
- `--out <salida.json>`: obligatorio.

## Qué no hace

- No edita valores de arquetipos o traits; para eso hay que cambiar el fichero y volver a validarlo.
- No genera propuestas nuevas ni analiza kits.
- No escribe en SQLite: la importación pasa por la app y por `profiles_import_set`.
- No convierte un conjunto incompleto en completo: la cobertura desconocida se mantiene visible.
