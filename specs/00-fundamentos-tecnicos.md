# SPEC 00 — Fundamentos técnicos

> **Estado:** Aprobada
> **Fecha:** 2026-08-31
> **Objetivo:** Disponer de una aplicación de escritorio Windows que arranque de forma confiable, prepare su almacenamiento SQLite local mediante migraciones versionadas y valide automáticamente su base técnica antes de incorporar módulos de negocio.
> **Depende de:** Ninguna
> **Modifica:** Ninguna
> **Reemplaza:** Ninguna

## Contexto

El proyecto parte de una plantilla Tauri 2 con React 19, TypeScript, Rust, SeaORM y SQLite. Actualmente la aplicación conserva la pantalla de demostración de Tauri, abre `inventory.db` mediante una ruta relativa, mantiene una base vacía dentro del repositorio y termina el proceso si falla la conexión. No existen migraciones, contrato común de errores, pruebas automatizadas ni integración continua.

El MVP se ejecutará como una aplicación de escritorio en un único PC Windows. Todos los usuarios funcionales del sistema compartirán la instalación y la base local desde una misma cuenta de Windows. No habrá servidor, sincronización entre equipos ni dependencia de Internet.

Esta SPEC establece la infraestructura común sobre la que dependerán autenticación, catálogo y movimientos de inventario, pero no introduce todavía entidades ni operaciones del dominio.

## Alcance

### Incluye

- Mantener Tauri 2, React, TypeScript, Rust, SeaORM y SQLite como base tecnológica.
- Resolver la ruta de la base desde el directorio de datos de la aplicación asignado por Tauri.
- Crear el directorio y la base automáticamente durante el primer inicio.
- Incorporar migraciones SQLite versionadas y ejecutarlas al iniciar la aplicación.
- Configurar SQLite para integridad referencial, espera ante bloqueos y operación local confiable.
- Centralizar en Rust todo acceso a SQLite y exponer funcionalidad al frontend únicamente mediante comandos Tauri.
- Definir un contrato serializable y estable para los errores enviados por los comandos Tauri.
- Sustituir la demostración de Tauri por un shell mínimo con estados de carga, listo y error de inicio.
- Permitir reintentar la inicialización después de un error recuperable, sin borrar datos automáticamente.
- Establecer una política offline con recursos empaquetados y permisos Tauri mínimos.
- Incorporar registro técnico local sin datos sensibles.
- Configurar formato, análisis estático, pruebas frontend/backend y compilación de la aplicación.
- Ejecutar las validaciones automáticas en Pull Requests mediante integración continua para Windows.
- Documentar los comandos de desarrollo, validación y la ubicación lógica de los datos locales.

### No incluye

- Usuarios, autenticación, sesiones o roles.
- Tablas de categorías, unidades, productos, proveedores, compras o movimientos.
- Navegación hacia módulos funcionales del inventario.
- Un servidor HTTP o una API accesible desde otros equipos.
- Sincronización, colaboración o uso simultáneo desde varios PC.
- Compatibilidad con macOS, Linux, dispositivos móviles o navegadores como objetivo del MVP.
- Copias de seguridad, restauración, importación o exportación de la base.
- Cifrado de la base de datos en reposo.
- Telemetría, analítica, actualizaciones automáticas o servicios externos.
- Firma de código, publicación del instalador o automatización de releases.
- Diseño visual definitivo del producto.

## Comportamiento esperado

### Inicio y almacenamiento local

1. Al abrir la aplicación, el backend obtiene el directorio de datos correspondiente a `com.remolinodelpez.inventario` mediante las APIs de rutas de Tauri; no depende del directorio desde el que se ejecutó el programa.
2. Si el directorio o la base no existen, el sistema los crea automáticamente.
3. Antes de declarar la aplicación lista, el backend abre SQLite, aplica su configuración y ejecuta en orden todas las migraciones pendientes.
4. Ejecutar varias veces la misma versión de la aplicación no vuelve a aplicar migraciones ya registradas ni altera datos existentes.
5. Cuando la inicialización termina, la interfaz muestra el nombre del producto, su versión y un estado listo. No muestra todavía opciones de inventario.
6. Mientras se consulta el estado inicial, la interfaz presenta un estado de carga y evita acciones dependientes de la base.
7. Si no puede resolverse la ruta, crearse el directorio, abrirse la base o completarse una migración, la ventana permanece abierta y muestra un error bloqueante en español con una acción para reintentar.
8. El reintento repite la inicialización sobre la misma ubicación. Nunca elimina, reemplaza ni reinicia automáticamente una base que no puede abrirse.
9. Los detalles técnicos del fallo se registran localmente, pero la interfaz no muestra trazas, consultas SQL ni rutas completas.

### Base de datos

1. Las claves foráneas quedan habilitadas en cada conexión.
2. Las conexiones esperan durante un intervalo acotado cuando SQLite está temporalmente ocupado, en vez de fallar de inmediato.
3. La estrategia de journal permite lecturas confiables durante escrituras locales y queda definida de forma uniforme para desarrollo y producción.
4. Las migraciones se ejecutan de manera transaccional cuando SQLite lo permita. Un fallo no debe registrar como completada una migración incompleta.
5. La base no puede consultarse directamente desde React ni quedar expuesta mediante permisos SQL del frontend.
6. En esta SPEC la única estructura persistida será la necesaria para controlar las versiones de migración; las tablas del dominio se incorporarán en sus respectivas SPECs.

### Shell de escritorio

1. Se eliminan de la experiencia visible el saludo, los logos y los enlaces de demostración de Vite, React y Tauri.
2. La ventana conserva el nombre `Inventario Remolino del Pez` y presenta estados diferenciables de carga, listo y error.
3. La interfaz funciona con recursos locales empaquetados y no solicita fuentes, scripts, estilos, imágenes ni datos a Internet.
4. El estado de error ofrece `Reintentar` y mantiene la aplicación utilizable para un nuevo intento sin reiniciar el proceso manualmente.
5. La interfaz es operable con teclado y los mensajes de estado son anunciables mediante semántica accesible básica.

### Errores y registros

1. Todo comando Tauri expuesto al frontend devuelve los errores esperados con el contrato común definido en esta SPEC.
2. La interfaz decide el mensaje y la acción a partir de un código estable; no analiza textos internos de Rust o SQLite.
3. Los errores inesperados usan un código genérico y conservan su causa completa únicamente en el registro técnico local.
4. Los registros incluyen fecha, severidad y contexto de operación, pero no contraseñas, tokens ni contenido completo de futuros registros de negocio.
5. La ausencia o imposibilidad de escritura del registro técnico no provoca pérdida ni modificación de la base de inventario.

### Validación automática

1. El repositorio proporciona comandos reproducibles para validar frontend y backend desde un entorno limpio.
2. Las pruebas de persistencia utilizan bases temporales y no leen ni modifican la base real del usuario.
3. Cada Pull Request ejecuta en Windows la instalación reproducible de dependencias, formato, análisis estático, pruebas, compilación frontend y validación del backend Tauri.
4. Una validación fallida produce un estado fallido en CI y señala el comando que no terminó correctamente.
5. La aplicación puede compilarse como aplicación Tauri para Windows sin requerir firma ni publicación.

## Modelo de datos y contratos

### Ubicación persistente

| Elemento | Contrato |
|---|---|
| Plataforma del MVP | Windows de escritorio |
| Topología | Un PC, una instalación y una base local |
| Directorio | Directorio de datos de aplicación resuelto por Tauri |
| Identificador | `com.remolinodelpez.inventario` |
| Archivo | `inventory.db` dentro del directorio de datos |
| Propietario del acceso | Backend Rust mediante SeaORM |
| Acceso frontend | Exclusivamente comandos Tauri |

La ubicación física concreta puede variar entre versiones de Windows y perfiles de usuario; por ello no se codifica una ruta absoluta. Dos cuentas distintas de Windows tendrán directorios y bases independientes.

### Estado de inicialización

El frontend debe poder distinguir, como mínimo, estos estados:

| Estado | Significado | Acción permitida |
|---|---|---|
| `loading` | Se está consultando o preparando el almacenamiento | Esperar |
| `ready` | La conexión y las migraciones finalizaron | Continuar al shell |
| `error` | La inicialización no pudo completarse | Reintentar |

La respuesta satisfactoria del estado incluye la versión de la aplicación. El estado de error utiliza el contrato común de errores y no expone información interna de la máquina.

### Contrato común de error

| Campo | Tipo | Requerido | Regla |
|---|---|---:|---|
| `code` | string | Sí | Identificador estable en `SCREAMING_SNAKE_CASE` |
| `message` | string | Sí | Mensaje seguro y comprensible en español |
| `details` | objeto | No | Solo información estructurada segura para orientar una corrección |

Códigos mínimos de esta SPEC:

| Código | Situación |
|---|---|
| `APP_DATA_DIR_UNAVAILABLE` | No se pudo determinar o preparar el directorio de datos |
| `DATABASE_UNAVAILABLE` | No se pudo abrir o comprobar la base |
| `DATABASE_MIGRATION_FAILED` | Una migración no pudo completarse |
| `INTERNAL_ERROR` | Fallo inesperado sin un código más específico |

Los nombres concretos de comandos y tipos internos pueden definirse durante la implementación, pero deben respetar estos estados, campos y códigos observables.

## Migración y compatibilidad

- `src-tauri/inventory.db` está vacío, no contiene objetos de esquema y no representa información de usuario. Debe dejar de versionarse y no se copiará a la ubicación definitiva.
- El primer inicio posterior a esta SPEC crea una base nueva en el directorio de datos de la aplicación y registra la versión inicial de migraciones.
- No existe información previa que deba migrarse desde la ruta relativa actual.
- Las siguientes SPECs solo modificarán el esquema mediante nuevas migraciones versionadas; no editarán migraciones que ya hayan sido distribuidas.
- Si una migración falla, la aplicación conserva el archivo existente, informa el error y permite reintentar. No realiza recuperación destructiva automática.
- Durante desarrollo y pruebas se permiten rutas inyectadas o bases temporales para aislamiento, pero una compilación de producción siempre utiliza el directorio resuelto por Tauri.

## Seguridad y privacidad

- El backend Rust es la única frontera con acceso a SQLite; el frontend no recibe credenciales ni capacidad para ejecutar SQL arbitrario.
- La política de contenido de producción solo admite recursos incluidos en la aplicación y las capacidades Tauri se reducen a las estrictamente necesarias.
- Se retiran permisos y dependencias de apertura externa que no sean utilizados por el shell.
- El modo de desarrollo puede permitir únicamente la conexión local requerida por Vite y su recarga en caliente; esa excepción no se traslada a producción.
- La aplicación no inicia solicitudes de red ni incorpora telemetría.
- Los errores visibles y registros evitan exponer datos sensibles. El cifrado del archivo SQLite queda explícitamente fuera de esta SPEC.

## Decisiones

- **DEC-01 — Aplicación local para un único PC Windows.** El MVP se ejecutará como escritorio nativo en un solo equipo porque la arquitectura elegida usa SQLite local y no requiere coordinación remota.
- **DEC-02 — Datos en el directorio de aplicación.** `inventory.db` se resolverá mediante las APIs de Tauri para evitar depender del directorio de trabajo, del repositorio o de permisos de instalación.
- **DEC-03 — SeaORM como acceso exclusivo a datos.** Se conserva la decisión técnica ya presente en el repositorio y se impide el acceso SQL desde React para mantener validaciones y transacciones en Rust.
- **DEC-04 — Migraciones versionadas desde el inicio.** El esquema evolucionará mediante el mecanismo de migraciones compatible con SeaORM; esto permite instalar y actualizar sin recrear manualmente la base.
- **DEC-05 — Inicialización recuperable.** Un fallo de almacenamiento se representa dentro de la ventana y admite reintento, en lugar de cerrar el proceso o borrar datos.
- **DEC-06 — Contrato estructurado de errores.** Los comandos usarán códigos estables y mensajes seguros porque los textos de librerías no constituyen una interfaz confiable para React.
- **DEC-07 — Funcionamiento offline y privilegio mínimo.** Los recursos se empaquetan localmente y se restringen CSP y capacidades Tauri para reducir superficie de ataque y dependencias operativas.
- **DEC-08 — Calidad continua en Windows.** Formato, análisis, pruebas y compilación se ejecutarán localmente y en Pull Requests para detectar regresiones antes de incorporar módulos de negocio.
- **Descartada — Base junto al ejecutable o en el repositorio.** Puede quedar en una ubicación sin permisos de escritura, duplicarse según el directorio de inicio o incluirse accidentalmente en control de versiones.
- **Descartada — SQLite accesible directamente desde React.** Duplicaría reglas entre capas y ampliaría innecesariamente los permisos del frontend.
- **Descartada — Servidor local o remoto para el MVP.** Añadiría despliegue, autenticación de red y sincronización sin aportar valor al escenario acordado de un solo PC.
- **Descartada — Compatibilidad multiplataforma inmediata.** El restaurante utilizará Windows; otros sistemas se evaluarán después del MVP si existe una necesidad concreta.

## Plan de implementación

### Bloque 1 — Persistencia y migraciones

- [ ] Sustituir la ruta relativa por la ruta del directorio de datos resuelta mediante Tauri y crear el directorio cuando falte.
- [ ] Incorporar el ejecutor de migraciones versionadas compatible con SeaORM y una migración inicial sin entidades de negocio.
- [ ] Aplicar de forma uniforme la configuración de integridad referencial, journal y espera ante bloqueos.
- [ ] Separar la inicialización de la construcción del proceso para poder conservar la ventana y reintentar después de fallos recuperables.
- [ ] Retirar la base vacía del control de versiones e impedir que nuevas bases locales sean añadidas accidentalmente.
- [ ] Añadir pruebas backend con bases temporales para creación inicial, reapertura, idempotencia de migraciones y fallos controlados.

**Resultado verificable:** una instalación sin base crea y prepara `inventory.db` en el directorio de datos; reiniciarla conserva la misma base y no repite migraciones.

### Bloque 2 — Contratos y observabilidad

- [ ] Definir el tipo serializable común de errores y la conversión controlada de fallos de rutas, SQLite y migraciones.
- [ ] Exponer al frontend el estado de preparación y la versión de la aplicación.
- [ ] Exponer una operación de reintento que no elimine ni reemplace el archivo existente.
- [ ] Centralizar las invocaciones Tauri del frontend para conservar tipos y tratamiento uniforme de errores.
- [ ] Incorporar registro técnico local con niveles, contexto y filtrado de información sensible.
- [ ] Cubrir con pruebas los contratos satisfactorios, los códigos de error y el reintento.

**Resultado verificable:** el frontend puede determinar de manera tipada si la aplicación está lista y puede reintentar un fallo usando códigos estables, sin recibir errores internos sin procesar.

### Bloque 3 — Shell de escritorio offline

- [ ] Retirar la interfaz y recursos de demostración que no pertenezcan al producto.
- [ ] Construir el shell mínimo con nombre, versión y estados accesibles de carga, listo y error.
- [ ] Añadir la acción de reintento y bloquear funciones dependientes de datos mientras la aplicación no esté lista.
- [ ] Ajustar la ventana y los estilos base para una experiencia de escritorio Windows utilizable con teclado.
- [ ] Restringir la política de contenido y las capacidades Tauri a recursos locales y permisos necesarios.
- [ ] Retirar dependencias y permisos externos sin uso.
- [ ] Probar los tres estados visibles y la acción de reintento en el frontend.

**Resultado verificable:** al abrir la aplicación se muestra el shell de Inventario Remolino del Pez; un fallo simulado de SQLite presenta un error recuperable y ninguna pantalla solicita recursos de Internet.

### Bloque 4 — Herramientas y validación continua

- [ ] Configurar un ejecutor de pruebas frontend compatible con React y TypeScript, incluyendo pruebas de componentes.
- [ ] Añadir comandos únicos y documentados para formato, análisis estático, pruebas y compilación del frontend y de Rust.
- [ ] Configurar formato y análisis estricto para Rust y mantener la comprobación estricta existente de TypeScript.
- [ ] Crear un flujo de CI para Pull Requests sobre Windows con instalación reproducible de Node.js y Rust.
- [ ] Ejecutar en CI las pruebas frontend/backend, formato, análisis estático, compilación web y validación de la aplicación Tauri.
- [ ] Documentar requisitos, inicio en desarrollo, ejecución de pruebas, compilación y ubicación lógica de datos.

**Resultado verificable:** un Pull Request limpio obtiene todas las validaciones exitosas y un fallo intencional de formato, tipos o pruebas hace fallar el control correspondiente.

## Criterios de aceptación

- [ ] **CA-01:** En un perfil de Windows sin base previa, la aplicación crea `inventory.db` dentro del directorio de datos resuelto por Tauri y alcanza el estado listo.
- [ ] **CA-02:** La base se abre correctamente aunque la aplicación se inicie desde directorios de trabajo diferentes.
- [ ] **CA-03:** Un segundo inicio reutiliza la misma base y no vuelve a ejecutar migraciones ya registradas.
- [ ] **CA-04:** Las conexiones verificadas tienen claves foráneas habilitadas y la configuración acordada de espera y journal.
- [ ] **CA-05:** Ningún componente React puede ejecutar SQL arbitrario ni necesita permisos de acceso directo a SQLite.
- [ ] **CA-06:** Un fallo simulado de apertura de la base mantiene la ventana abierta y muestra un mensaje bloqueante en español con la acción `Reintentar`.
- [ ] **CA-07:** Reintentar después de corregir un fallo recuperable lleva la aplicación al estado listo sin borrar ni reemplazar la base.
- [ ] **CA-08:** Los fallos de directorio, conexión y migración llegan al frontend con los códigos definidos y sin trazas, SQL o rutas completas visibles.
- [ ] **CA-09:** La interfaz ya no muestra logos, enlaces, saludo ni textos de demostración de Tauri, Vite o React.
- [ ] **CA-10:** El shell diferencia visual y semánticamente los estados de carga, listo y error, y la acción de reintento puede operarse con teclado.
- [ ] **CA-11:** Una compilación de producción no solicita recursos remotos y no conserva permisos Tauri externos sin uso.
- [ ] **CA-12:** Las pruebas de persistencia crean bases temporales y dejan intacta la base ubicada en el directorio real de la aplicación.
- [ ] **CA-13:** Existen comandos documentados que validan formato, análisis estático, pruebas y compilación de TypeScript y Rust desde un entorno limpio.
- [ ] **CA-14:** Cada Pull Request ejecuta las validaciones acordadas en un runner Windows y falla si cualquiera de ellas no se completa.
- [ ] **CA-15:** La aplicación Tauri para Windows compila sin requerir firma, publicación, servidor externo ni conexión funcional a Internet.
- [ ] **CA-16:** `src-tauri/inventory.db` deja de estar versionado y el primer inicio no intenta importarlo ni copiarlo.

## Riesgos

| Riesgo | Mitigación |
|---|---|
| Una ruta por perfil de Windows genere bases distintas si se usan varias cuentas del sistema operativo | Documentar que el MVP opera bajo una sola cuenta de Windows y mostrar posteriormente la ubicación de datos en soporte técnico |
| Una migración fallida impida iniciar los módulos futuros | Migraciones versionadas, transacciones cuando sean compatibles, registro local y reintento no destructivo |
| Un bloqueo temporal de SQLite se confunda con corrupción | Configurar espera acotada, distinguir códigos de error y no eliminar la base automáticamente |
| Diferencias entre desarrollo con Vite y producción empaquetada oculten permisos de red | Mantener excepciones locales solo en desarrollo y validar en CI una compilación Tauri de producción |
| Los logs expongan datos del inventario en futuras funcionalidades | Centralizar el registro, excluir datos sensibles por defecto y revisar cada nuevo contexto registrado |
