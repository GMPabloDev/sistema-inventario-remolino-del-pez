# SPEC 01 — Autenticación, usuarios y roles

> **Estado:** En implementación
> **Fecha:** 2026-09-02
> **Objetivo:** Permitir que los usuarios autorizados inicien y cierren sesión de forma local, que un administrador gestione cuentas y que cada comando protegido aplique los roles `ADMIN` y `WAREHOUSE_MANAGER`.
> **Depende de:** [SPEC 00 — Fundamentos técnicos](00-fundamentos-tecnicos.md)
> **Modifica:** Ninguna
> **Reemplaza:** Ninguna

## Contexto

La aplicación ya prepara una base SQLite local mediante migraciones, centraliza el acceso a datos en Rust y expone al frontend un contrato común de errores. Todavía no existen usuarios, sesiones ni autorización, por lo que cualquier persona con acceso a la aplicación alcanza el shell sin identificarse y las futuras operaciones de inventario no podrían registrar un responsable confiable.

El MVP funciona offline en un único PC Windows y bajo una misma cuenta del sistema operativo. La autenticación será local, sin servidor HTTP, proveedor de identidad ni recuperación por correo. Los “endpoints protegidos” descritos por el MVP corresponden en esta arquitectura a comandos Tauri cuya autorización debe comprobar el backend Rust.

## Alcance

### Incluye

- Persistir usuarios identificados internamente mediante UUID y autenticados con un nombre de usuario único.
- Admitir únicamente los roles `ADMIN` y `WAREHOUSE_MANAGER`.
- Crear de forma segura el primer usuario `ADMIN` cuando la base todavía no contenga usuarios.
- Mostrar una contraseña temporal aleatoria para el primer administrador y exigir su cambio antes de habilitar el resto del sistema.
- Iniciar sesión, restaurar una sesión recordada, cerrar sesión y cambiar la contraseña propia.
- Recordar una sesión durante un máximo de siete días mediante una credencial opaca protegida por el sistema operativo.
- Aplicar espera progresiva ante intentos de acceso fallidos y evitar revelar si un usuario existe, está inactivo o tiene una contraseña incorrecta.
- Autorizar cada comando protegido en Rust y exponer al frontend únicamente las acciones permitidas para la sesión vigente.
- Permitir a un `ADMIN` crear, listar y actualizar usuarios, restablecer contraseñas y activar o desactivar cuentas.
- Impedir el borrado físico de usuarios y cualquier operación que deje al sistema sin un `ADMIN` activo.
- Invalidar sesiones cuando una cuenta se desactive, cambie de rol o tenga su contraseña restablecida.
- Incorporar las pantallas y estados necesarios para bootstrap, login, cambio obligatorio de contraseña, sesión autenticada y gestión de usuarios.
- Añadir migraciones, pruebas backend/frontend y documentación de las reglas de autenticación.

### No incluye

- Registro público, invitaciones por correo ni autoservicio para crear cuentas.
- Recuperación de contraseña por email, SMS, preguntas de seguridad o servicios externos.
- Inicio de sesión con correo electrónico, UUID, cuenta de Windows, OAuth, biometría o proveedores externos.
- Autenticación multifactor.
- Roles personalizados, permisos individuales o edición de la matriz de permisos desde la interfaz.
- Autenticación entre equipos, API HTTP, sincronización o sesiones para varios dispositivos.
- Cifrado del archivo SQLite en reposo.
- Eliminación física de usuarios.
- Auditoría funcional avanzada o una pantalla de historial de eventos administrativos.
- Gestión de empleados, datos laborales o perfiles personales distintos del nombre visible.
- Permisos detallados de módulos que serán definidos por specs posteriores; esta SPEC establece el mecanismo común y los permisos de administración de usuarios.

## Comportamiento esperado

### Inicio y administrador inicial

1. Después de que la base alcance el estado `ready` definido en SPEC 00, la aplicación determina en Rust si existe una sesión recordada válida y si ya hay usuarios configurados.
2. Si la tabla de usuarios está vacía, el backend crea en una transacción un usuario activo con nombre de usuario `admin`, rol `ADMIN`, UUID nuevo y obligación de cambiar contraseña.
3. La contraseña temporal del primer administrador es aleatoria, tiene al menos 20 caracteres y se muestra en la interfaz sin escribirse en la base, logs, portapapeles ni almacenamiento web.
4. La interfaz advierte que la contraseña temporal debe copiarse por un medio seguro y que dejará de ser válida cuando se complete el cambio obligatorio.
5. Si la aplicación se cierra antes de completar el primer cambio de contraseña, el siguiente inicio reemplaza la credencial temporal pendiente por una nueva y vuelve a mostrarla. La contraseña anterior deja de ser válida.
6. El bootstrap es idempotente: nunca crea un segundo administrador mientras exista cualquier usuario y nunca reemplaza las credenciales de una instalación ya configurada.
7. Una sesión autenticada con contraseña temporal solo puede consultar su identidad, cambiar esa contraseña o cerrar sesión. Ningún módulo ni comando administrativo queda habilitado antes del cambio.

### Inicio y cierre de sesión

1. El login solicita nombre de usuario y contraseña.
2. El nombre de usuario se recorta y normaliza a minúsculas antes de buscarlo. La contraseña se trata como un secreto exacto y no se recorta ni transforma.
3. Usuario inexistente, contraseña incorrecta y usuario inactivo producen el mismo código y mensaje visible de credenciales inválidas.
4. Después de tres intentos fallidos consecutivos para un mismo identificador durante la ejecución actual, el backend aplica esperas progresivas de 1, 2, 4 y 8 segundos, con un máximo de 30 segundos por intento. Un login exitoso reinicia el contador.
5. La espera ocurre en el backend y no puede omitirse invocando directamente el comando Tauri. No bloquea permanentemente la cuenta ni impide que otro usuario inicie sesión.
6. Un login correcto devuelve únicamente la identidad segura del usuario y establece la sesión activa; nunca devuelve el hash de contraseña ni el secreto de sesión a React.
7. Si el usuario debe cambiar su contraseña, la interfaz muestra directamente el flujo obligatorio y no el shell funcional.
8. Al cerrar sesión, el backend revoca la sesión, elimina la credencial recordada y devuelve la interfaz al login.
9. Abrir la aplicación sin una sesión válida muestra el login. Una sesión vencida, revocada o perteneciente a un usuario inactivo se descarta y muestra un aviso seguro para volver a iniciar sesión.
10. Un nuevo login en la instalación reemplaza la sesión recordada anterior. Todas las ventanas del mismo proceso comparten la misma sesión activa.

### Sesión recordada

1. La sesión utiliza un secreto aleatorio opaco de al menos 256 bits; no utiliza JWT porque no existe un servidor remoto que necesite verificar afirmaciones autocontenidas.
2. SQLite conserva únicamente un hash no reversible del secreto, el usuario asociado, las fechas de creación y expiración y su estado de revocación.
3. El secreto necesario para restaurar la sesión se guarda mediante el almacenamiento seguro de credenciales de Windows. No se guarda en `localStorage`, archivos de configuración, logs ni texto plano dentro de SQLite.
4. La expiración absoluta ocurre siete días después del login y no se extiende automáticamente por actividad.
5. Cada comando protegido valida en Rust que la sesión exista, no esté vencida ni revocada, que el usuario siga activo y que conserve el rol requerido.
6. Cambiar la contraseña propia, restablecer una contraseña, cambiar un rol o desactivar un usuario revoca todas las sesiones de esa cuenta.
7. Si el almacenamiento seguro de Windows no está disponible, el sistema no persiste el secreto de forma insegura: mantiene la sesión solo durante el proceso actual y muestra una advertencia de que no podrá restaurarla al reiniciar.

### Contraseñas

1. Toda contraseña elegida por un usuario tiene entre 12 y 128 caracteres.
2. Se permiten espacios y caracteres Unicode; no se exigen reglas arbitrarias de mayúsculas, números o símbolos.
3. La nueva contraseña debe diferir de la contraseña vigente cuando esta pueda verificarse durante el cambio.
4. Las contraseñas se almacenan exclusivamente como hashes Argon2id con sal aleatoria y parámetros versionados. Nunca se persisten ni registran en texto plano.
5. El backend rechaza entradas superiores al límite antes de ejecutar el hash para acotar el consumo de recursos.
6. El cambio de contraseña propia requiere la contraseña actual, salvo cuando la sesión está marcada para cambio obligatorio mediante una credencial temporal válida.
7. Una contraseña temporal generada al crear o restablecer una cuenta se muestra una sola vez en esa operación, obliga a cambiarla en el siguiente login e invalida cualquier contraseña o sesión anterior.

### Gestión de usuarios

1. Solo un `ADMIN` con contraseña definitiva puede abrir y ejecutar la gestión de usuarios.
2. La lista muestra UUID, nombre de usuario, nombre visible, rol, estado y fechas de creación y actualización; nunca muestra hashes, contraseñas temporales ni secretos de sesión.
3. Al crear un usuario, el administrador define nombre de usuario, nombre visible y rol. La cuenta nace activa con UUID nuevo y una contraseña temporal aleatoria mostrada una sola vez.
4. El administrador puede editar el nombre de usuario, el nombre visible y el rol de otra cuenta. Cambiar el nombre de usuario o el rol invalida sus sesiones.
5. El administrador puede activar una cuenta inactiva. La reactivación no restaura sesiones ni contraseñas temporales vencidas.
6. Al desactivar una cuenta, todas sus sesiones se revocan inmediatamente y los futuros intentos de login se rechazan con el mensaje genérico de credenciales inválidas.
7. Restablecer una contraseña genera una nueva contraseña temporal, marca el cambio como obligatorio y revoca todas las sesiones del usuario.
8. El usuario autenticado no puede cambiar su propio rol, nombre de usuario ni estado desde la administración. Sí puede cambiar su propia contraseña y el nombre visible cuando exista una pantalla de perfil.
9. Ninguna creación, actualización o desactivación puede dejar al sistema sin al menos un usuario activo con rol `ADMIN`. La comprobación y el cambio se realizan en una misma transacción.
10. Los usuarios no se eliminan físicamente. Sus UUID permanecen disponibles para las relaciones y la trazabilidad que incorporen specs posteriores.
11. Dos administradores que intenten crear o renombrar usuarios al mismo nombre reciben un resultado determinista: solo una operación puede confirmar y la otra obtiene un error de duplicado.

### Validaciones de usuario

| Campo         | Regla observable                                                                                                                          |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `id`          | UUID generado por el backend; nunca se introduce manualmente ni se usa para el login                                                      |
| `username`    | Entre 3 y 32 caracteres normalizados a minúsculas; solo letras ASCII, números, punto, guion y guion bajo; único sin distinguir mayúsculas |
| `displayName` | Entre 1 y 100 caracteres después de recortar espacios exteriores                                                                          |
| `role`        | Uno de `ADMIN` o `WAREHOUSE_MANAGER`                                                                                                      |
| `active`      | Booleano; las cuentas nuevas nacen activas                                                                                                |
| `password`    | Entre 12 y 128 caracteres; no se recorta ni se devuelve al frontend después de enviarla                                                   |

Los mensajes de formularios pueden indicar reglas de formato y duplicados durante la administración. El login siempre conserva el mensaje genérico para no revelar el estado de una cuenta.

### Roles y autorización

| Capacidad                                                        |         `ADMIN`          |   `WAREHOUSE_MANAGER`    | Sin sesión |
| ---------------------------------------------------------------- | :----------------------: | :----------------------: | :--------: |
| Consultar estado técnico de inicio y reintentar una base fallida |            Sí            |            Sí            |     Sí     |
| Iniciar o restaurar sesión                                       |            Sí            |            Sí            |     Sí     |
| Consultar identidad propia                                       |            Sí            |            Sí            |     No     |
| Cambiar contraseña propia y cerrar sesión                        |            Sí            |            Sí            |     No     |
| Listar, crear y editar usuarios                                  |            Sí            |            No            |     No     |
| Restablecer contraseñas y activar/desactivar usuarios            |            Sí            |            No            |     No     |
| Acceder a futuros comandos de inventario                         | Según la spec del módulo | Según la spec del módulo |     No     |

1. Ocultar botones en React mejora la experiencia, pero no constituye autorización. Rust debe rechazar cada invocación no permitida.
2. Todo comando de negocio incorporado por specs posteriores será protegido por defecto y deberá declarar los roles admitidos.
3. Solo los comandos estrictamente necesarios para preparar la aplicación, mostrar el bootstrap, iniciar/restaurar sesión y resolver un cambio obligatorio pueden ejecutarse sin una sesión funcional completa.
4. Los comandos técnicos residuales o de demostración que no sean necesarios no permanecen como una vía pública sin autorización.
5. Una sesión cuyo rol cambió no conserva permisos anteriores, incluso si una vista del frontend todavía no se actualizó.

### Estados de interfaz y errores

1. La secuencia visible de inicio distingue: preparación de base, comprobación de autenticación, bootstrap inicial, login, cambio obligatorio de contraseña y shell autenticado.
2. Mientras se restaura una sesión o ejecuta una acción, los controles implicados quedan deshabilitados para evitar envíos duplicados y se comunica el estado de carga.
3. El login permite enviar con teclado, conserva el nombre de usuario después de un error y limpia el campo de contraseña.
4. El shell autenticado muestra el nombre visible, el rol y una acción accesible para cerrar sesión.
5. La gestión de usuarios contempla estados de carga, lista vacía, error recuperable, validación de formulario y confirmación antes de desactivar o restablecer credenciales.
6. Las contraseñas temporales se presentan en una vista que advierte que solo se mostrarán en esa operación; no vuelven a aparecer al recargar una lista.
7. La interfaz de `WAREHOUSE_MANAGER` no muestra la administración de usuarios. Una invocación manual recibe igualmente `AUTH_FORBIDDEN`.
8. Los errores usan el contrato común de SPEC 00 y códigos estables:

| Código                          | Situación                                                              |
| ------------------------------- | ---------------------------------------------------------------------- |
| `AUTH_INVALID_CREDENTIALS`      | Usuario inexistente, inactivo o contraseña incorrecta durante el login |
| `AUTH_SESSION_REQUIRED`         | Un comando protegido se invocó sin sesión                              |
| `AUTH_SESSION_EXPIRED`          | La sesión superó su vigencia de siete días                             |
| `AUTH_FORBIDDEN`                | El rol vigente no autoriza la operación                                |
| `AUTH_PASSWORD_CHANGE_REQUIRED` | La sesión temporal intentó ejecutar otra operación                     |
| `AUTH_PERSISTENCE_UNAVAILABLE`  | Windows no permitió recordar la credencial de sesión de forma segura   |
| `PASSWORD_VALIDATION_FAILED`    | La nueva contraseña incumple sus reglas                                |
| `USERNAME_ALREADY_EXISTS`       | El nombre normalizado ya pertenece a otro usuario                      |
| `USER_VALIDATION_FAILED`        | Uno o más campos de usuario son inválidos                              |
| `USER_NOT_FOUND`                | Una operación administrativa referencia un UUID inexistente            |
| `LAST_ACTIVE_ADMIN_REQUIRED`    | La operación dejaría al sistema sin un `ADMIN` activo                  |
| `SELF_MANAGEMENT_NOT_ALLOWED`   | El administrador intenta alterar su propio rol, usuario o estado       |

## Modelo de datos y contratos

### Usuario

| Campo                | Tipo lógico   | Requerido | Regla                                                       |
| -------------------- | ------------- | :-------: | ----------------------------------------------------------- |
| `id`                 | UUID          |    Sí     | Clave primaria inmutable generada por el backend            |
| `username`           | string        |    Sí     | Valor canónico en minúsculas y único                        |
| `displayName`        | string        |    Sí     | Nombre mostrado en la interfaz                              |
| `passwordHash`       | string        |    Sí     | Hash Argon2id; nunca serializado al frontend                |
| `role`               | enum          |    Sí     | `ADMIN` o `WAREHOUSE_MANAGER`                               |
| `active`             | boolean       |    Sí     | Controla acceso sin borrar historial                        |
| `mustChangePassword` | boolean       |    Sí     | Restringe la sesión hasta definir una contraseña definitiva |
| `passwordChangedAt`  | timestamp UTC |    No     | Fecha del último cambio definitivo                          |
| `createdAt`          | timestamp UTC |    Sí     | Generada por el backend                                     |
| `updatedAt`          | timestamp UTC |    Sí     | Actualizada por el backend                                  |

La base aplica la unicidad de `username` y restringe los valores de `role`. Las operaciones que protegen al último administrador se validan transaccionalmente en Rust.

### Sesión

| Campo       | Tipo lógico   | Requerido | Regla                                                         |
| ----------- | ------------- | :-------: | ------------------------------------------------------------- |
| `id`        | UUID          |    Sí     | Identificador interno de la sesión                            |
| `userId`    | UUID          |    Sí     | Referencia al usuario sin borrado en cascada                  |
| `tokenHash` | string        |    Sí     | Hash del secreto opaco; único y nunca serializado             |
| `createdAt` | timestamp UTC |    Sí     | Inicio de la sesión                                           |
| `expiresAt` | timestamp UTC |    Sí     | Siete días después de `createdAt`                             |
| `revokedAt` | timestamp UTC |    No     | Presente cuando la sesión dejó de ser válida antes de expirar |

Las sesiones vencidas o revocadas pueden depurarse posteriormente sin eliminar usuarios ni información de negocio.

### Identidad expuesta al frontend

| Campo                | Tipo                         | Regla                                                         |
| -------------------- | ---------------------------- | ------------------------------------------------------------- |
| `id`                 | UUID                         | Identificador para trazabilidad y operaciones administrativas |
| `username`           | string                       | Nombre canónico de acceso                                     |
| `displayName`        | string                       | Nombre visible                                                |
| `role`               | `ADMIN \| WAREHOUSE_MANAGER` | Rol vigente, no una lista editable de permisos                |
| `mustChangePassword` | boolean                      | Determina si solo se permite el cambio obligatorio            |

React no recibe `passwordHash`, `tokenHash`, la contraseña temporal después de su entrega única ni detalles del almacenamiento seguro del sistema operativo.

## Migración y compatibilidad

- La migración inicial de SPEC 00 permanece sin cambios. Esta SPEC añade una migración versionada posterior para usuarios y sesiones.
- Las bases existentes no contienen usuarios de dominio; después de migrar, el primer inicio ejecuta el bootstrap transaccional del administrador.
- La creación del usuario `admin` no forma parte de la migración SQL para que su contraseña aleatoria pueda entregarse de forma controlada a la interfaz.
- Reabrir una base ya configurada conserva usuarios, hashes y sesiones; no vuelve a ejecutar el bootstrap.
- Un fallo durante la migración o el bootstrap no registra un estado parcial como completado ni borra la base. La aplicación conserva el flujo recuperable de SPEC 00.
- Una actualización fallida del almacenamiento seguro de Windows no provoca que el secreto de sesión se copie a un medio inseguro. La cuenta y su contraseña permanecen válidas para un nuevo login.
- No existe compatibilidad que mantener con JWT, cookies, usuarios anteriores ni contraseñas en texto plano porque esos contratos todavía no existen.

## Seguridad y privacidad

- El hashing, la verificación de contraseñas, la generación de secretos, el estado de sesión y toda decisión de autorización se ejecutan en Rust.
- Los parámetros de Argon2id se centralizan, quedan versionados y permiten rehash futuro después de un login correcto sin cambiar el contrato visible.
- Las comparaciones de secretos y hashes usan las primitivas seguras de la biblioteca elegida; no se implementa criptografía manual.
- Contraseñas, hashes, secretos de sesión y credenciales temporales se excluyen de serialización, mensajes de error y logs.
- Los registros técnicos pueden indicar tipo de evento, fecha y UUID conocido después de autenticar, pero no guardan la contraseña, el token ni el identificador presentado en un login fallido.
- La aplicación no revela mediante mensajes si un nombre de usuario existe o está inactivo.
- Las sesiones se comprueban en cada comando protegido para que desactivaciones, cambios de rol y revocaciones tengan efecto inmediato.
- El frontend no almacena secretos de autenticación y no puede convertir la visibilidad de una opción en un permiso.
- La sesión recordada confía en la protección del perfil de Windows del escenario definido en SPEC 00. El cifrado completo de `inventory.db` continúa fuera de alcance.

## Decisiones

- **DEC-01 — UUID interno y nombre de usuario para el acceso.** El UUID mantiene relaciones y trazabilidad aunque cambie el nombre de acceso; el nombre de usuario es práctico en una aplicación offline y no exige correo.
- **DEC-02 — Dos roles cerrados.** Solo existen `ADMIN` y `WAREHOUSE_MANAGER` porque el MVP no necesita permisos individuales ni un editor de roles.
- **DEC-03 — Administrador inicial con credencial temporal aleatoria.** El sistema crea `admin`, muestra una contraseña no predecible y obliga a cambiarla para evitar una cuenta inicial sin acceso o un secreto fijo distribuido.
- **DEC-04 — Sesión opaca recordada durante siete días.** Un token aleatorio con estado en backend permite revocación inmediata y restauración local; su secreto se protege mediante el almacén de credenciales de Windows.
- **DEC-05 — Autorización obligatoria en Rust.** La ocultación de controles en React no protege comandos Tauri, por lo que el backend valida sesión, estado y rol en cada operación.
- **DEC-06 — Contraseñas con Argon2id y reglas de longitud.** Se usa un algoritmo específico para contraseñas, sal aleatoria y límites claros, sin reglas de composición que fomenten patrones previsibles.
- **DEC-07 — Espera progresiva sin bloqueo permanente.** Los intentos repetidos se ralentizan desde el backend para reducir fuerza bruta local sin permitir que un tercero inutilice indefinidamente una cuenta.
- **DEC-08 — Desactivación en lugar de eliminación.** Los UUID deben permanecer disponibles para la trazabilidad futura de compras y movimientos.
- **DEC-09 — Protección del último administrador.** Las operaciones administrativas nunca pueden dejar cero cuentas `ADMIN` activas y el usuario actual no puede alterar su propio rol, nombre de acceso o estado.
- **Descartada — Usar UUID como credencial de acceso.** Es adecuado como clave interna, pero demasiado largo y difícil de recordar para el login cotidiano.
- **Descartada — JWT.** Añade afirmaciones autocontenidas y gestión de claves sin aportar valor en una aplicación local sin API remota; además dificulta la revocación inmediata.
- **Descartada — Contraseña inicial fija.** Un secreto conocido o documentado permitiría acceder a cualquier instalación que no haya completado el cambio.
- **Descartada — Guardar sesión en `localStorage` o SQLite en texto plano.** Expondría una credencial reutilizable a la capa web o a una lectura directa del archivo.
- **Descartada — Borrar usuarios.** Rompería las referencias y la atribución histórica exigidas por las futuras operaciones de inventario.

## Plan de implementación

### Bloque 1 — Dominio, migración y primitivas de seguridad

- [x] Incorporar las entidades de usuario y sesión con sus restricciones, índices y una migración posterior a la inicial.
- [x] Implementar normalización de nombres de usuario, validaciones y protección transaccional del último administrador.
- [x] Incorporar generación criptográfica de UUID, contraseñas temporales y secretos de sesión.
- [x] Centralizar hashing y verificación Argon2id con parámetros versionados y límites de entrada.
- [x] Cubrir migración, restricciones, reapertura y operaciones concurrentes mediante bases temporales.

**Resultado verificable:** una base existente migra sin perder datos, persiste usuarios y sesiones válidos y rechaza duplicados, roles inválidos o la pérdida del último administrador.

### Bloque 2 — Bootstrap, autenticación y sesión

- [x] Implementar el bootstrap idempotente de `admin` y la entrega controlada de su contraseña temporal.
- [x] Implementar login con error genérico, espera progresiva y cambio obligatorio de contraseña.
- [x] Implementar creación, restauración, expiración y revocación de la sesión recordada durante siete días.
- [x] Integrar el secreto persistente con el almacén seguro de credenciales de Windows y el fallback no persistente.
- [x] Implementar consulta de identidad, cambio de contraseña propia y logout.
- [x] Probar reinicios, expiración, revocación, usuario inactivo, credenciales erróneas y pérdida del almacén seguro.

**Resultado verificable:** el primer administrador establece una contraseña definitiva, una sesión válida se restaura tras reiniciar y cualquier sesión vencida o revocada vuelve al login sin exponer secretos.

### Bloque 3 — Autorización y administración de usuarios

- [x] Crear un mecanismo común para exigir sesión, cambio de contraseña completado y rol en comandos Tauri.
- [x] Clasificar los comandos públicos existentes y retirar o proteger comandos técnicos o de demostración innecesarios.
- [x] Implementar listado, creación y actualización de usuarios para `ADMIN`.
- [x] Implementar activación, desactivación y restablecimiento mediante contraseñas temporales de entrega única.
- [x] Revocar sesiones afectadas dentro de las mismas operaciones que cambian contraseña, rol o estado.
- [x] Probar cada operación con `ADMIN`, `WAREHOUSE_MANAGER`, sesión temporal, sesión revocada y ausencia de sesión.

**Resultado verificable:** invocar directamente un comando administrativo sin un `ADMIN` vigente siempre falla y los cambios de seguridad tienen efecto antes de confirmar la operación.

### Bloque 4 — Flujo React

- [ ] Extender el cliente tipado de comandos y códigos de error sin exponer secretos persistentes.
- [ ] Orquestar los estados de preparación, bootstrap, restauración, login, cambio obligatorio y shell autenticado.
- [ ] Crear formularios accesibles de login, cambio de contraseña y gestión de usuarios con validación visible.
- [ ] Mostrar contraseñas temporales únicamente en la respuesta inmediata de bootstrap, creación o restablecimiento.
- [ ] Adaptar navegación y acciones visibles al rol, manteniendo la autorización real en Rust.
- [ ] Cubrir estados de carga, error, expiración, permisos y acciones con pruebas de componentes.

**Resultado verificable:** cada usuario atraviesa el flujo correspondiente a su estado y rol mediante teclado, y ninguna recarga o reapertura vuelve a mostrar una credencial temporal ya entregada.

### Bloque 5 — Validación y documentación

- [ ] Añadir pruebas backend para hashes, límites, tiempos de espera, transacciones, sesiones y serialización segura.
- [ ] Añadir pruebas frontend para mensajes genéricos, limpieza de secretos, logout y administración autorizada.
- [ ] Verificar que logs, errores y respuestas no contengan contraseñas, hashes ni tokens.
- [ ] Documentar el acceso inicial, la duración de sesión, los roles y el procedimiento administrativo de restablecimiento.
- [ ] Ejecutar todas las validaciones frontend, Rust y la compilación Tauri definidas en SPEC 00.

**Resultado verificable:** CI valida los flujos críticos y una inspección automatizada de contratos y logs no encuentra secretos de autenticación.

## Criterios de aceptación

- [ ] **CA-01:** Una base migrada sin usuarios crea exactamente un `ADMIN` activo con UUID, usuario `admin` y contraseña temporal aleatoria de al menos 20 caracteres.
- [ ] **CA-02:** La contraseña temporal inicial se muestra sin persistirse en texto plano y, si no se completa el cambio antes de cerrar, el siguiente inicio entrega una nueva e invalida la anterior.
- [ ] **CA-03:** Una sesión con contraseña temporal solo permite consultar la identidad, cambiar la contraseña o cerrar sesión.
- [ ] **CA-04:** Un nombre de usuario inexistente, una contraseña incorrecta y una cuenta inactiva muestran el mismo error de credenciales inválidas.
- [ ] **CA-05:** A partir del cuarto fallo consecutivo para un identificador, el backend aplica la espera progresiva definida, con máximo de 30 segundos, y un login correcto reinicia el contador.
- [ ] **CA-06:** Una contraseña definitiva menor de 12 o mayor de 128 caracteres se rechaza; una válida se almacena únicamente como hash Argon2id con sal aleatoria.
- [ ] **CA-07:** Un login válido crea una sesión cuyo secreto no llega a React ni se guarda en `localStorage`, archivos de configuración o SQLite en texto plano.
- [ ] **CA-08:** Una sesión recordada válida restaura la identidad después de reiniciar la aplicación y vence exactamente siete días después de su creación.
- [ ] **CA-09:** Logout, cambio o restablecimiento de contraseña, cambio de rol y desactivación impiden reutilizar inmediatamente cualquier sesión anterior del usuario afectado.
- [ ] **CA-10:** Si Windows no permite guardar el secreto de sesión de forma segura, el login funciona solo para el proceso actual, informa la limitación y no crea un fallback inseguro.
- [ ] **CA-11:** Un `ADMIN` puede crear un usuario con cualquiera de los dos roles y recibe una contraseña temporal visible una sola vez.
- [ ] **CA-12:** Nombres como `Manager`, `manager` y `MANAGER` se consideran el mismo usuario para la restricción de unicidad.
- [ ] **CA-13:** Un `ADMIN` puede listar y editar otras cuentas, restablecer sus contraseñas y activarlas o desactivarlas.
- [ ] **CA-14:** Ningún usuario puede cambiar desde la administración su propio rol, nombre de acceso o estado.
- [ ] **CA-15:** Ninguna operación puede desactivar o degradar al último `ADMIN` activo, incluso ante solicitudes concurrentes.
- [ ] **CA-16:** Un usuario desactivado conserva su registro y UUID, no puede iniciar sesión y no recupera sesiones anteriores al reactivarse.
- [ ] **CA-17:** `WAREHOUSE_MANAGER`, una sesión temporal y una invocación sin sesión reciben un error estable al ejecutar cualquier comando de gestión de usuarios.
- [ ] **CA-18:** Ocultar o modificar controles en React no permite eludir la autorización aplicada por Rust.
- [ ] **CA-19:** El inicio de la aplicación diferencia preparación de base, autenticación, bootstrap, login, cambio obligatorio y shell autenticado sin mostrar brevemente contenido protegido.
- [ ] **CA-20:** La interfaz limpia la contraseña después de un login fallido y no vuelve a mostrar una credencial temporal después de abandonar su vista de entrega.
- [ ] **CA-21:** Errores, respuestas serializadas y logs no contienen contraseñas, hashes, secretos de sesión ni credenciales temporales.
- [ ] **CA-22:** Las pruebas usan bases y credenciales aisladas, y todas las validaciones de SPEC 00 continúan pasando en Windows.

## Riesgos

| Riesgo                                                                             | Mitigación                                                                                                                           |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| La credencial inicial se pierda antes de configurar el sistema                     | Regenerarla e invalidar la anterior en cada inicio mientras continúe pendiente el primer cambio                                      |
| Una contraseña temporal quede expuesta en pantalla                                 | Mostrarla solo durante la operación, no copiarla automáticamente ni registrarla y forzar su cambio                                   |
| El secreto de una sesión recordada sea extraído del equipo                         | Guardarlo mediante el almacén seguro de Windows, persistir solo su hash en SQLite y limitarlo a siete días                           |
| Una sesión conserve permisos después de un cambio administrativo                   | Validar usuario y rol en cada comando y revocar sesiones dentro de la misma transacción del cambio                                   |
| Dos operaciones de administración eliminen al último administrador por una carrera | Comprobar y actualizar dentro de una transacción con estrategia compatible con el bloqueo de SQLite                                  |
| Argon2id haga lenta la interfaz en equipos modestos                                | Seleccionar parámetros medidos para el equipo objetivo, ejecutar el trabajo fuera del hilo de interfaz y mantener límites de entrada |
| Reiniciar la aplicación permita evitar la espera progresiva                        | Documentar que el contador protege la ejecución actual y reevaluar persistencia o bloqueo temporal durante el hardening del MVP      |
| Una persona con acceso completo al perfil de Windows lea la base sin cifrar        | Mantener hashes resistentes y secretos fuera de SQLite; el cifrado en reposo continúa fuera del alcance acordado                     |
