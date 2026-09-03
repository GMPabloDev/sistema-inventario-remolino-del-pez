# Inventario Remolino del Pez

Aplicación de escritorio para el control de inventario de un restaurante. El MVP está construido con Tauri 2, React, TypeScript, Rust, SeaORM y SQLite.

## Requisitos

- Windows 10 u 11.
- Node.js `24.19.0` y npm.
- Rust `1.98.0`, con `rustfmt` y `clippy`.
- WebView2 Runtime para ejecutar la aplicación Tauri.

Las versiones de Node.js y Rust están fijadas en `.nvmrc` y `rust-toolchain.toml`.

## Desarrollo

```bash
npm ci
npm run tauri dev
```

La aplicación funciona offline en ejecución. Vite utiliza `http://localhost:1420` únicamente durante el desarrollo.

## Validaciones locales

Ejecuta desde la raíz del proyecto:

```bash
npm run format:check
npm run typecheck
npm test
npm run build

cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

Para corregir el formato frontend:

```bash
npm run format
```

La integración continua ejecuta estas validaciones y la compilación Tauri en cada Pull Request sobre Windows.

## Compilación de escritorio

```bash
npm run tauri build -- --debug
```

El instalador y el ejecutable se generan dentro de `src-tauri/target/`. La firma y la publicación quedan fuera del MVP.

## Datos locales

SQLite se guarda en el directorio de datos de la aplicación resuelto por Tauri:

```text
<directorio de datos de la aplicación>/inventory.db
<directorio de datos de la aplicación>/inventory.log
```

La ruta física depende del perfil de Windows y no debe codificarse en el proyecto. La base `src-tauri/inventory.db` no se utiliza ni se versiona.

El acceso a SQLite ocurre exclusivamente en Rust mediante SeaORM. Las migraciones se ejecutan automáticamente durante el inicio y se registran en `seaql_migrations`.

## Acceso inicial y autenticación

En una instalación sin usuarios, el primer inicio crea el usuario `admin` con rol `ADMIN` y muestra una contraseña temporal aleatoria una sola vez. Inicia sesión con ella y define una contraseña definitiva antes de continuar.

Las contraseñas definitivas deben tener entre 12 y 128 caracteres y se almacenan como hashes Argon2id. Las sesiones recordadas duran siete días; su secreto se guarda en Windows Credential Manager y SQLite conserva únicamente su hash. Si el almacén seguro no está disponible, la sesión se mantiene solo durante el proceso y no se usa un fallback inseguro.

Los roles disponibles son `ADMIN` y `WAREHOUSE_MANAGER`. Solo `ADMIN` puede gestionar usuarios. Puede crear, editar, activar, desactivar y restablecer cuentas; los restablecimientos generan una contraseña temporal de entrega única. Las cuentas se desactivan sin borrarse físicamente y nunca se permite eliminar al último administrador activo.

## Estructura inicial

- `src/`: interfaz React y cliente de comandos Tauri.
- `src-tauri/src/`: backend Rust, persistencia, migraciones y comandos.
- `src-tauri/capabilities/`: permisos de la ventana Tauri.
- `specs/`: especificaciones SDD del proyecto.
- `.github/workflows/ci.yml`: validación automática de Pull Requests.
