# 10 - Modo SoloQ (sincronizado con el cliente)

> Estado: implementado y verificado, incluida la lectura del cliente de League of
> Legends en una partida real: el draft se rellena solo y el pick propio se
> registra en cuanto el cliente lo bloquea. La entrada manual sigue disponible
> como respaldo y la pantalla no depende de que el cliente esté abierto.

## 1. Para qué sirve

El simulador de la Fase 1 reproduce el draft competitivo (6 bans, 6 picks, 4
bans, 4 picks). En ranked la estructura es otra: 5 baneos por equipo y una sola
fase de picks con el orden `1-2-2-2-2-1`. El modo SoloQ permite describir ese
estado en segundos y responder a una única pregunta:

> Dado lo que ya está bloqueado, ¿qué campeón encaja mejor en **mi** turno?

No es un visor de winrates. La respuesta es el mismo índice de ajuste
estratégico (0-100) que usa el simulador competitivo, con su desglose exacto y
sus motivos.

## 2. Flujo de uso

No hay que preparar nada: al abrir **SoloQ** la app empieza a leer el cliente
cada dos segundos.

1. Si el cliente está en selección de campeones, lado, turno, rol asignado,
   picks y baneos aparecen ya rellenos.
2. Cuando el cliente bloquea tu campeón, la tarjeta **Tu pick** pasa a
   «Bloqueado · turno N». Ese campeón se excluye de los candidatos y el botón
   «Ver mejor pick» se desactiva: ya no queda pick que recomendar.
3. Mientras sigues eligiendo, pulsa **Ver mejor pick** para el análisis del
   turno actual.

La barra de estado resume lo que está pasando sin ruido:

| Estado | Significado |
| --- | --- |
| `Cliente conectado · fase …` | Leyendo un champ select real. Se actualiza solo. |
| `Cliente cerrado` | No hay `lockfile`; se puede rellenar a mano. |
| `Cliente abierto, fuera de champ select` | Esperando a que empiece el draft. |
| `Sincronización pausada` | El usuario la ha parado con el botón. |
| `No se pudo hablar…` | El cliente responde mal; el draft anterior no se borra. |

Controles:

- **● Sincronizando / ○ Sincronización pausada** alterna la lectura automática.
  Al pausarla, el draft visible se conserva y se puede editar a mano.
- **Actualizar** fuerza una lectura inmediata y anuncia el resultado, con los
  campeones del cliente que no existen en el catálogo local.
- **Limpiar** vacía el draft, pero la siguiente lectura vuelve a traer lo que
  mande el cliente.

Solo se aplica un estado cuando **cambia**: si el cliente repite la misma
respuesta cada dos segundos, el draft y las recomendaciones no se tocan. Los
fallos de lectura (cliente cerrado, fuera de champ select) solo cambian la línea
de estado; nunca vacían el draft ya leído.

El estado **no se persiste**: es una consulta, no una sesión de draft. Al
cambiar cualquier dato de entrada el resultado anterior se descarta, para no
mostrar una recomendación calculada sobre otro estado.

### 2.1 Baneos opcionales

Los baneos son opcionales porque en ranked pueden no conocerse todavía. La
interfaz avisa de la consecuencia: si se omiten, un campeón ya baneado puede
aparecer como recomendación, porque el motor solo excluye lo que recibe.

## 3. Qué hace el motor con este formato

`DraftFormat::soloq()` describe 10 baneos alternos y 10 picks con el orden real
de ranked. Se usa en memoria; no se guarda en SQLite ni cambia el formato
estándar.

Las fases no se deducen de índices fijos, sino de la estructura del formato:

| Fase | Regla |
| --- | --- |
| `early_ban` | baneo antes del primer pick |
| `late_ban` | baneo después de algún pick |
| `early_pick` | pick entre los 6 primeros picks globales |
| `late_pick` | pick a partir del séptimo pick global |

Con eso, el pick 1 de SoloQ usa la tabla de pesos de pick temprano y el pick 10
la de pick final, igual que sus equivalentes competitivos. La tabla de pesos no
se duplica ni se recalibra: es la misma hipótesis documentada en
[`04-motor-recomendaciones.md`](./04-motor-recomendaciones.md).

El pick solicitado se evalúa con `pick_action_index(side, slot)`, es decir, con
el índice de acción que ese turno ocupa en el formato. Así el contexto rival
revelado y la fase son coherentes con el orden real.

### 3.1 Rol fijo

Si el jugador conoce su posición, se envía `fixed_role`. El motor solo evalúa
campeones cuya proficiencia en ese rol supera el mínimo; el resto se descarta en
vez de reasignarse. Los aliados se reparten los roles **reservando** el del
jugador (`assign_roles_avoiding`), para que un flex aliado no ocupe por
heurística el rol que se está evaluando.

Sin rol fijo se evalúan todos los roles libres del equipo aliado.

Nota: si un aliado ya bloqueado solo puede ocupar el rol reservado, el análisis
lo deja sin rol y no lo cuenta en la composición, igual que en el simulador
competitivo. No se le inventa una posición para que encaje.

## 4. Lectura del cliente

`loldraft-sources::lcu` lee el cliente por su API local no documentada:

- localiza el `lockfile` (`LOLDRAFT_LCU_LOCKFILE` o rutas habituales de
  instalación) y extrae puerto y contraseña;
- consulta `GET /lol-champ-select/v1/session` con autenticación básica y
  certificado autofirmado aceptado solo para `127.0.0.1`;
- normaliza lado, turno, posición asignada, picks completados, **tu pick** y
  baneos.

Es **solo lectura**: no envía acciones al cliente ni escribe nada. Solo se
consideran picks con `completed = true`; un hueco no se rellena con un campeón
inventado. `own_pick` se separa de los aliados: la tarjeta «Tu pick» lo muestra
como bloqueado y el motor no lo propone. Los campeones del cliente que no
existen en el catálogo local se omiten y se informan en la interfaz.

Limitaciones conocidas:

- la API local no es pública y puede cambiar entre parches;
- si el `lockfile` no existe, el cliente no está abierto o no está en selección,
  la app lo explica en la línea de estado y mantiene el draft y la entrada
  manual;
- la lectura es de solo lectura: la app no banea ni bloquea por ti.

Códigos de error estables: `lcu_not_running`, `lcu_not_in_champ_select`,
`lcu_no_pick_action`, `lcu_unreachable`, `lcu_invalid_session`.

## 5. Límites y honestidad de los datos

- No se inventan picks ni roles: un campeón sin perfil en el conjunto activo no
  aparece puntuado.
- Los perfiles disponibles son los del conjunto activo (el piloto de 38
  campeones, marcado como `inferred`); el resto simplemente no se propone.
- Si no hay conjunto de perfiles activo, la pantalla lo dice y no calcula nada.
- La lista se diversifica por rol y arquetipo, igual que en competitivo.
- El índice no es una probabilidad de victoria y la interfaz lo repite.

## 6. Errores de entrada

| Código | Cuándo |
| --- | --- |
| `soloq_invalid_slot` | turno fuera de 1-5 o inexistente en el formato |
| `soloq_duplicate_champion` | el mismo campeón aparece dos veces |
| `soloq_unknown_champion` | clave que no existe en el catálogo local |
| `recommendation_error` | el motor rechaza la solicitud |

El duplicado se rechaza en lugar de "arreglarse" en silencio: un estado
imposible no produce una recomendación fiable.

## 7. Verificación

- Dominio: formato SoloQ, mapeo de turnos a índices y reserva de rol (23 tests).
- Motor: fases equivalentes entre formatos y filtrado por rol fijo (73 tests).
- Fuentes: `lockfile`, sesión azul/roja, turnos posteriores, pick propio
  bloqueado, espectador y posiciones asignadas (12 tests).
- UI: compilación TypeScript y verificación contra cliente real: el draft se
  rellena solo, el pick propio aparece al bloquearlo sin tocar nada, la pausa
  conserva el draft y una caída del cliente no lo borra.
