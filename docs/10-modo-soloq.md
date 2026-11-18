# 10 - Modo SoloQ (draft manual)

> Estado: implementado y verificado en la app con draft manual. La importación
> desde el cliente de League of Legends está implementada pero **no verificada en
> vivo**: en el entorno de desarrollo no había un cliente en selección de
> campeones. Se marca como experimental en la interfaz.

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

1. Elegir lado (azul/rojo), turno de pick (1-5) y, opcionalmente, rol.
2. Rellenar los picks aliados y rivales ya bloqueados.
3. Rellenar los baneos (opcional, pero recomendado).
4. Pulsar **Ver mejor pick**.

La pantalla destaca el primer candidato y muestra la lista completa de
recomendaciones reutilizando el panel de la Fase 3: contribuciones, riesgos,
features sin datos y enlaces al perfil del campeón.

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

## 4. Importación desde el cliente (experimental)

`loldraft-sources::lcu` lee el cliente por su API local no documentada:

- localiza el `lockfile` (`LOLDRAFT_LCU_LOCKFILE` o rutas habituales de
  instalación) y extrae puerto y contraseña;
- consulta `GET /lol-champ-select/v1/session` con autenticación básica y
  certificado autofirmado aceptado solo para `127.0.0.1`;
- normaliza lado, turno, posición asignada, picks completados y baneos.

Es **solo lectura**: no envía acciones al cliente ni escribe nada. Solo se
consideran picks con `completed = true`; un hueco no se rellena con un campeón
inventado. Los campeones del cliente que no existen en el catálogo local se
omiten y se informan en la interfaz.

Limitaciones conocidas:

- la API local no es pública y puede cambiar entre parches;
- no se ha podido probar contra un cliente real en este entorno, solo con
  fixtures sintéticos de la respuesta;
- si el `lockfile` no existe, el cliente no está abierto o no está en selección,
  la app muestra un error específico y sigue permitiendo la entrada manual.

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
- Fuentes: `lockfile`, sesión azul/roja, turnos posteriores, espectador y
  posiciones asignadas (11 tests).
- UI: compilación TypeScript y prueba manual del flujo en la app.
