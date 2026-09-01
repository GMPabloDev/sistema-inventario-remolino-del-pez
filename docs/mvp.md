# Sistema de Control de Inventario para Restaurante — MVP

## 1. Definición del MVP

**MVP** significa **Minimum Viable Product**, en español **Producto Mínimo Viable**.

Es la primera versión funcional de un sistema que incluye únicamente lo necesario para resolver el problema principal y demostrar que realmente funciona.

En este proyecto, el problema principal es:

> Controlar correctamente el inventario de un restaurante, registrando entradas y salidas de productos, mostrando el stock actual y manteniendo trazabilidad de cada movimiento.

El objetivo inicial **no** es construir un ERP completo, un POS, un sistema contable ni una plataforma integral de gestión de restaurante.

La prioridad es construir primero un núcleo pequeño, funcional, demostrable y estable.

---

# 2. Objetivo principal del sistema

El sistema debe permitir:

- Iniciar sesión.
- Gestionar productos e insumos.
- Gestionar categorías.
- Gestionar proveedores.
- Registrar entradas de inventario.
- Registrar salidas de inventario.
- Consultar el stock actual.
- Consultar un Kardex básico.
- Detectar productos con stock bajo.
- Mostrar un dashboard sencillo.
- Mantener trazabilidad sobre quién realizó cada movimiento.

---

# 3. Flujo mínimo que debe demostrar el MVP

El flujo principal será:

```text
Login
  ↓
Productos
  ↓
Registrar entrada
  ↓
Aumenta stock
  ↓
Registrar salida
  ↓
Disminuye stock
  ↓
Consultar Kardex
```

Ejemplo:

```text
Producto: Arroz

Stock inicial: 20
Entrada: +10
Salida: -5

Stock actual: 25
```

Si el sistema puede ejecutar correctamente este flujo y mantener el historial de movimientos, el núcleo del inventario funciona.

---

# 4. Roles del sistema

Para el MVP se utilizarán únicamente dos roles.

## ADMIN

Representa al administrador del sistema.

Puede:

- Gestionar usuarios.
- Gestionar categorías.
- Gestionar productos.
- Gestionar proveedores.
- Registrar entradas.
- Registrar salidas.
- Registrar ajustes.
- Consultar Kardex.
- Consultar dashboard.
- Consultar stock bajo.
- Activar/desactivar registros.
- Acceder a funciones administrativas.

---

## WAREHOUSE_MANAGER

Representa al **Encargado de almacén**.

Puede:

- Consultar productos.
- Consultar stock.
- Registrar entradas.
- Registrar consumos.
- Registrar mermas.
- Consultar Kardex.
- Consultar alertas de stock bajo.
- Consultar dashboard.

En el MVP no es necesario implementar un sistema complejo de permisos individuales.

---

# 5. Conceptos principales

## 5.1 Producto / insumo

Representa cualquier elemento que el restaurante necesita controlar.

Ejemplos:

- Pollo
- Carne
- Arroz
- Papa
- Aceite
- Cebolla
- Gaseosas
- Agua
- Envases
- Productos de limpieza

Cada producto tendrá al menos:

```text
id
name
description
categoryId
unit
currentStock
minimumStock
cost
active
createdAt
updatedAt
```

---

# 6. Categorías

Permiten organizar los productos.

Ejemplos:

```text
Carnes
Verduras
Granos
Bebidas
Limpieza
Empaques
```

Funciones:

- Crear categoría.
- Editar categoría.
- Listar categorías.
- Activar/desactivar categoría.

---

# 7. Unidades de medida

Ejemplos:

```text
kg
g
L
ml
unidad
paquete
caja
```

Cada producto debe tener una unidad base.

Ejemplos:

```text
Arroz → kg
Aceite → L
Huevos → unidad
```

Para el MVP no es obligatorio implementar conversiones complejas como:

```text
kg ↔ g
L ↔ ml
caja ↔ unidad
```

---

# 8. Entradas de inventario

Una **entrada** es cualquier movimiento que aumenta el stock.

Ejemplos:

- Compra a proveedor.
- Reposición.
- Devolución al almacén.
- Ajuste positivo.

Ejemplo:

```text
Stock actual: 10 kg de pollo

Entrada: +20 kg

Nuevo stock: 30 kg
```

---

# 9. Salidas de inventario

Una **salida** es cualquier movimiento que reduce el stock.

Ejemplos:

- Consumo en cocina.
- Merma.
- Producto vencido.
- Producto dañado.
- Pérdida.
- Ajuste negativo.

Ejemplo:

```text
Stock actual: 30 kg de pollo

Salida: -8 kg

Nuevo stock: 22 kg
```

---

# 10. Tipos de movimiento

Tipos iniciales recomendados:

```text
PURCHASE
CONSUMPTION
WASTE
ADJUSTMENT_IN
ADJUSTMENT_OUT
```

Cada movimiento debe registrar como mínimo:

```text
id
productId
type
quantity
previousStock
newStock
reason
userId
createdAt
```

---

# 11. Proveedores

Los proveedores representan las empresas o personas que suministran productos al restaurante.

Datos recomendados:

```text
id
name
documentNumber
phone
email
address
active
```

Funciones:

- Crear proveedor.
- Editar proveedor.
- Consultar proveedor.
- Listar proveedores.
- Activar/desactivar proveedor.

No forman parte del MVP:

- Portal de proveedores.
- Evaluación de proveedores.
- Cuentas por pagar.
- Facturación electrónica.

---

# 12. Compras / entradas

Una compra puede contener múltiples productos.

Ejemplo:

```text
Proveedor: Distribuidora ABC

Compra:
- 20 kg de pollo
- 30 kg de arroz
- 10 L de aceite
```

Entidades sugeridas:

```text
Purchase
PurchaseItem
```

## Purchase

```text
id
supplierId
documentNumber
date
total
notes
createdBy
createdAt
```

## PurchaseItem

```text
id
purchaseId
productId
quantity
unitCost
subtotal
```

Al registrar una compra, el sistema debe:

1. Crear la compra.
2. Crear sus detalles.
3. Incrementar el stock de cada producto.
4. Crear un movimiento `PURCHASE`.
5. Registrar el usuario responsable.

La operación debe ejecutarse dentro de una transacción de base de datos.

---

# 13. Consumos

Representan productos utilizados normalmente por el restaurante.

Ejemplo:

```text
Producto: Pollo
Cantidad: 5 kg
Motivo: Consumo cocina turno noche
```

Tipo:

```text
CONSUMPTION
```

---

# 14. Mermas

Representan productos que dejan de estar disponibles sin haber sido utilizados normalmente.

Ejemplos:

- Producto vencido.
- Producto quemado.
- Derrame.
- Rotura.
- Deterioro.

Tipo:

```text
WASTE
```

Cada merma debe registrar:

- Producto.
- Cantidad.
- Motivo.
- Usuario responsable.
- Fecha.

---

# 15. Ajustes básicos de inventario

Los ajustes permiten corregir diferencias conocidas de stock.

Ejemplo:

```text
Stock sistema: 18 kg
Stock real conocido: 16 kg

Ajuste:
-2 kg
```

Tipo:

```text
ADJUSTMENT_OUT
```

Otro ejemplo:

```text
Stock sistema: 18 kg
Stock real conocido: 20 kg

Ajuste:
+2 kg
```

Tipo:

```text
ADJUSTMENT_IN
```

## En el MVP

Los ajustes serán simples.

Inicialmente solo `ADMIN` podrá realizarlos directamente.

Todo ajuste debe registrar:

- Producto.
- Stock anterior.
- Cantidad ajustada.
- Stock resultante.
- Motivo.
- Usuario.
- Fecha.

## Fuera del MVP

La aprobación formal de ajustes solicitados por el encargado de almacén queda para una fase posterior.

---

# 16. Stock actual

Cada producto tendrá un campo:

```text
currentStock
```

El sistema debe mostrar el stock disponible actual del producto.

Ejemplo:

```text
Arroz
Stock actual: 25 kg
Stock mínimo: 10 kg
```

---

# 17. Regla crítica: no permitir stock negativo

En condiciones normales:

```text
cantidadSalida <= stockActual
```

Por tanto:

```text
stockActual - cantidadSalida >= 0
```

Ejemplo:

```text
Stock actual: 5 kg
Salida solicitada: 8 kg
```

Resultado:

```text
OPERACIÓN RECHAZADA
```

El sistema no debe permitir:

```text
Stock: -3 kg
```

---

# 18. Kardex básico

El Kardex permite consultar la trazabilidad de un producto.

Ejemplo:

```text
Producto: Pollo

31/08  +20 kg   Compra        Stock: 30 kg
31/08   -5 kg   Consumo       Stock: 25 kg
31/08   -2 kg   Merma         Stock: 23 kg
31/08   +1 kg   Ajuste        Stock: 24 kg
```

Filtros recomendados:

- Producto.
- Tipo de movimiento.
- Fecha desde.
- Fecha hasta.
- Usuario.
- Entrada/salida.

Cada movimiento debe mostrar:

- Fecha.
- Producto.
- Tipo.
- Cantidad.
- Stock anterior.
- Stock resultante.
- Responsable.
- Motivo.
- Referencia a compra cuando corresponda.

---

# 19. Regla crítica: movimientos inmutables

Los movimientos históricos no deberían editarse ni eliminarse.

Si existe un error, debe generarse un movimiento correctivo.

Ejemplo:

```text
Movimiento incorrecto:
-10 kg

Corrección:
+10 kg
```

De esta forma se conserva la trazabilidad.

---

# 20. Stock mínimo

Cada producto tendrá:

```text
minimumStock
```

Si:

```text
currentStock <= minimumStock
```

el producto se considera:

```text
LOW_STOCK
```

Si:

```text
currentStock = 0
```

se considera:

```text
OUT_OF_STOCK
```

---

# 21. Alertas de stock bajo

En el MVP las alertas serán visuales dentro del sistema.

Ejemplo:

```text
Producto      Stock      Mínimo      Estado
Pollo         4 kg       10 kg       STOCK BAJO
Arroz         5 kg        8 kg       STOCK BAJO
Aceite        0 L         5 L        SIN STOCK
```

No se necesitan inicialmente:

- Emails.
- WhatsApp.
- Push notifications.
- SMS.

---

# 22. Dashboard sencillo

El dashboard debe ofrecer una vista rápida del inventario.

Indicadores recomendados:

```text
Productos activos
Productos con stock bajo
Productos sin stock
Valor estimado del inventario
Entradas recientes
Salidas recientes
```

Ejemplo:

```text
Productos activos:        128
Stock bajo:                 12
Sin stock:                   3
Valor inventario:      S/ 14,820
```

También debe mostrar:

## Productos con stock bajo

```text
Pollo       4 kg       Mínimo: 10 kg
Arroz       5 kg       Mínimo: 8 kg
Aceite      2 L        Mínimo: 5 L
```

## Últimos movimientos

```text
+20 kg   Pollo      Compra
-5 kg    Arroz      Consumo
-2 kg    Papa       Merma
```

---

# 23. Valor estimado del inventario

Para el MVP puede calcularse:

```text
stock actual × costo actual
```

Ejemplo:

```text
Pollo
20 kg × S/ 12.00 = S/ 240.00
```

No es necesario inicialmente implementar:

- FIFO.
- LIFO.
- Promedio ponderado avanzado.
- Valoración contable completa.

---

# 24. Módulos del MVP

El MVP estará compuesto por:

```text
1. Autenticación
2. Usuarios y roles
3. Categorías
4. Unidades de medida
5. Productos
6. Proveedores
7. Compras / entradas
8. Consumos
9. Mermas
10. Ajustes básicos
11. Stock actual
12. Kardex básico
13. Alertas de stock bajo
14. Dashboard sencillo
```

---

# 25. Orden recomendado de SPECs

## SPEC 01 — Autenticación y roles

Implementar:

- Usuarios.
- Login.
- Logout.
- JWT o sesión elegida.
- Roles:
  - `ADMIN`
  - `WAREHOUSE_MANAGER`
- Protección de endpoints.
- Usuarios activos/inactivos.

### Motivo

Todo movimiento debe poder relacionarse con el usuario responsable.

---

# SPEC 02 — Catálogo base de inventario

Implementar:

- Categorías.
- Unidades de medida.
- Productos.
- Stock actual.
- Stock mínimo.
- Costo.
- Estado activo/inactivo.

### Motivo

Todas las operaciones de inventario dependen de productos existentes.

---

# SPEC 03 — Proveedores

Implementar:

- Crear proveedor.
- Editar proveedor.
- Consultar proveedor.
- Listar proveedores.
- Activar/desactivar proveedor.

### Motivo

Las compras necesitarán asociarse con proveedores.

---

# SPEC 04 — Compras y entradas de inventario

Implementar:

- `Purchase`.
- `PurchaseItem`.
- Registro de compras.
- Incremento de stock.
- Movimiento `PURCHASE`.
- Transacciones de base de datos.

### Motivo

Es la forma principal de ingresar stock.

---

# SPEC 05 — Consumos y mermas

Implementar:

```text
CONSUMPTION
WASTE
```

Incluir:

- Producto.
- Cantidad.
- Motivo.
- Usuario responsable.
- Fecha.
- Validación de stock disponible.
- Prevención de stock negativo.

### Motivo

Completa el flujo básico:

```text
ENTRADA
   ↓
INVENTARIO
   ↓
SALIDA
```

---

# SPEC 06 — Ajustes básicos de inventario

Implementar:

```text
ADJUSTMENT_IN
ADJUSTMENT_OUT
```

Reglas:

- Solo ADMIN.
- Motivo obligatorio.
- Stock anterior.
- Stock resultante.
- Movimiento inmutable.

---

# SPEC 07 — Kardex y trazabilidad

Implementar:

- Historial de movimientos.
- Consulta por producto.
- Filtros.
- Stock anterior.
- Stock resultante.
- Usuario responsable.
- Motivo.
- Referencia a compra cuando corresponda.

---

# SPEC 08 — Alertas de stock mínimo

Implementar:

- Productos bajo mínimo.
- Productos sin stock.
- Estado visual.
- Filtros.
- Contadores.

---

# SPEC 09 — Dashboard sencillo

Implementar:

- Productos activos.
- Productos con stock bajo.
- Productos sin stock.
- Valor aproximado del inventario.
- Últimos movimientos.
- Entradas recientes.
- Salidas recientes.

---

# SPEC 10 — Hardening y cierre del MVP

Revisar:

- Autorización.
- Validaciones.
- Transacciones.
- Concurrencia.
- Integridad de stock.
- Índices.
- Manejo de errores.
- Auditoría básica.
- Tests unitarios.
- Tests de integración.
- Seed/demo data.

---

# 26. Dependencias entre SPECs

```text
SPEC 01
Autenticación
    │
    ▼
SPEC 02
Productos / Categorías
    │
    ▼
SPEC 03
Proveedores
    │
    ▼
SPEC 04
Compras / Entradas
    │
    ▼
SPEC 05
Consumos / Mermas
    │
    ▼
SPEC 06
Ajustes
    │
    ▼
SPEC 07
Kardex
    │
    ▼
SPEC 08
Stock bajo
    │
    ▼
SPEC 09
Dashboard
    │
    ▼
SPEC 10
Hardening
```

---

# 27. Modelo de datos inicial sugerido

```text
User
Category
Product
Supplier
Purchase
PurchaseItem
InventoryMovement
```

Relaciones aproximadas:

```text
Category
   │
   └── Product
          │
          ├── PurchaseItem
          │
          └── InventoryMovement

Supplier
   │
   └── Purchase
          │
          └── PurchaseItem

User
   │
   ├── Purchase
   └── InventoryMovement
```

---

# 28. Reglas de negocio principales

## RB-01 — No permitir stock negativo

```text
stockActual - cantidadSalida >= 0
```

---

## RB-02 — Todo cambio de stock genera movimiento

Nunca modificar:

```text
currentStock
```

sin generar:

```text
InventoryMovement
```

---

## RB-03 — Guardar stock anterior y resultante

Ejemplo:

```text
previousStock = 20
quantity = -5
newStock = 15
```

---

## RB-04 — Movimientos históricos inmutables

No editar ni eliminar movimientos antiguos.

---

## RB-05 — Operaciones críticas transaccionales

Ejemplo:

```text
BEGIN

crear compra
crear detalles
actualizar stock
crear movimientos

COMMIT
```

Si algo falla:

```text
ROLLBACK
```

---

## RB-06 — Productos con historial no se eliminan físicamente

Usar:

```text
active = false
```

---

## RB-07 — Registrar usuario responsable

Todo movimiento debe tener:

```text
userId
```

---

## RB-08 — Cantidades y costos decimales

Para cantidades:

```text
1.5 kg
0.25 L
2.75 kg
```

usar tipos decimales adecuados.

Para valores monetarios evitar `float`.

Ejemplo conceptual:

```text
quantity DECIMAL
unitCost DECIMAL
```

---

# 29. Funcionalidades PLUS / Post-MVP

Estas funciones no son necesarias para demostrar el núcleo inicial.

Se desarrollarán únicamente después de cerrar el MVP.

---

## PLUS 01 — Inventario físico

Permitir realizar conteos físicos completos.

Ejemplo:

```text
Producto     Sistema     Conteo físico     Diferencia
Arroz        30 kg       28 kg             -2 kg
Aceite       12 L        13 L              +1 L
```

El inventario físico podrá generar propuestas de ajustes.

---

# PLUS 02 — Ajustes con aprobación del administrador

Flujo:

```text
Encargado de almacén
        ↓
Solicita ajuste
        ↓
Administrador revisa
        ↓
Aprueba / rechaza
        ↓
Si aprueba
        ↓
Se actualiza stock
```

Estados posibles:

```text
PENDING
APPROVED
REJECTED
```

---

# PLUS 03 — Reportes avanzados

Ejemplos:

```text
Consumo por período
Mermas por período
Productos más consumidos
Compras por proveedor
Entradas vs salidas
Valor del inventario
Movimientos por usuario
Productos con mayor merma
Variación de stock
```

---

# PLUS 04 — Auditoría avanzada

Registrar eventos administrativos adicionales.

Ejemplos:

- Usuario creado.
- Producto modificado.
- Producto desactivado.
- Proveedor modificado.
- Ajuste solicitado.
- Ajuste aprobado.
- Ajuste rechazado.
- Cambio de costo.
- Cambio de stock mínimo.

---

# PLUS 05 — Gráficos interactivos

Ejemplos:

- Entradas vs salidas.
- Consumo por semana.
- Mermas por mes.
- Valor del inventario.
- Productos más consumidos.
- Compras por proveedor.

---

# PLUS 06 — Exportación a Excel

Permitir exportar:

- Inventario.
- Kardex.
- Compras.
- Movimientos.
- Mermas.
- Reportes.

---

# PLUS 07 — Exportación a PDF

Permitir generar:

- Kardex PDF.
- Reporte de inventario.
- Reporte de compras.
- Reporte de mermas.
- Resumen mensual.

---

# PLUS 08 — Compras avanzadas

Agregar posteriormente:

- Estados de compra.
- Recepción parcial.
- Cancelaciones.
- Órdenes de compra.
- Observaciones avanzadas.
- Adjuntos.
- Comparación de precios.
- Historial de costos.

---

# PLUS 09 — Órdenes de compra

Permitir crear solicitudes antes de recibir productos.

Flujo:

```text
Stock bajo
   ↓
Orden de compra
   ↓
Proveedor
   ↓
Recepción
   ↓
Entrada de inventario
```

---

# PLUS 10 — Recetas

Ejemplo:

```text
Lomo Saltado

200 g carne
150 g papa
100 g arroz
50 g cebolla
30 g tomate
```

---

# PLUS 11 — Descuento automático por preparación o venta

Ejemplo:

```text
5 Lomos Saltados vendidos
```

El sistema podría descontar automáticamente:

```text
-1 kg carne
-750 g papa
-500 g arroz
-250 g cebolla
-150 g tomate
```

---

# PLUS 12 — Costeo de platos

Calcular:

```text
Costo del plato =
suma del costo de todos sus ingredientes
```

---

# PLUS 13 — Múltiples almacenes

Ejemplo:

```text
Almacén principal
Cocina
Bar
```

Permitir transferencias internas.

---

# PLUS 14 — Integración con POS

Una venta podría generar automáticamente movimientos de consumo.

---

# 30. Funcionalidades explícitamente fuera del MVP

Para evitar crecimiento innecesario del alcance, no se implementarán inicialmente:

- POS.
- Facturación electrónica.
- SUNAT.
- Contabilidad.
- Gestión de mesas.
- Reservas.
- Delivery.
- Gestión de empleados.
- Planillas.
- Múltiples restaurantes.
- Múltiples almacenes.
- Transferencias internas.
- Recetas.
- Costeo de platos.
- Descuento automático por ventas.
- Inventario físico avanzado.
- Aprobaciones de ajustes.
- Reportes avanzados.
- Auditoría avanzada.
- Gráficos interactivos.
- Exportación Excel.
- Exportación PDF.
- Órdenes de compra.
- Predicción de demanda.
- IA.
- WhatsApp.
- Emails automáticos.

---

# 31. Criterio para considerar terminado el MVP

El MVP se considera funcional cuando se pueda demostrar este escenario completo:

1. Un usuario ADMIN inicia sesión.
2. Crea una categoría.
3. Crea un producto.
4. Define su stock mínimo.
5. Crea un proveedor.
6. Registra una compra.
7. El stock aumenta correctamente.
8. El Encargado de almacén registra un consumo.
9. El stock disminuye correctamente.
10. Registra una merma.
11. El sistema conserva todos los movimientos.
12. Se consulta el Kardex del producto.
13. Se intenta registrar una salida superior al stock disponible.
14. El sistema rechaza la operación.
15. Se realiza un ajuste administrativo.
16. El producto alcanza el stock mínimo.
17. El sistema lo marca como stock bajo.
18. El dashboard refleja el estado actual.

Ejemplo final:

```text
Producto: Arroz

Stock inicial:     20 kg
Compra:           +10 kg
Consumo:           -4 kg
Merma:             -1 kg
────────────────────────
Stock actual:      25 kg
```

El Kardex debería mostrar:

```text
+10 kg   Compra
-4 kg    Consumo
-1 kg    Merma
```

Si este flujo funciona correctamente de extremo a extremo, el MVP cumple su objetivo.

---

# 32. Prioridades

## P0 — Obligatorio

```text
Autenticación
Roles
Categorías
Productos
Stock actual
Entradas
Salidas
Compras
Consumos
Mermas
Movimientos
Kardex
Prevención de stock negativo
```

## P1 — Importante para cerrar MVP

```text
Proveedores
Ajustes básicos
Stock mínimo
Alertas visuales
Dashboard sencillo
Hardening
Tests
```

## P2 — Post-MVP

```text
Inventario físico
Aprobación de ajustes
Reportes avanzados
Auditoría avanzada
Gráficos interactivos
Excel
PDF
Compras avanzadas
Órdenes de compra
```

## P3 — Evolución futura

```text
Recetas
Costeo de platos
Descuento automático
Múltiples almacenes
POS
Automatizaciones
IA
```

---

# 33. Nota para la creación de futuras SPECs

Cada SPEC debe indicar:

- Objetivo.
- Alcance.
- Fuera de alcance.
- Entidades afectadas.
- Roles.
- Permisos.
- Casos de uso.
- Reglas de negocio.
- Validaciones.
- Errores esperados.
- Transacciones necesarias.
- Criterios de aceptación.
- Tests requeridos.
- Dependencias.

Una SPEC no debería ampliar el alcance silenciosamente.

Si durante la implementación aparece una nueva funcionalidad, debe decidirse explícitamente si:

1. Es obligatoria para completar el MVP.
2. Debe convertirse en una nueva SPEC.
3. Pertenece al Post-MVP.
4. Queda fuera del proyecto.

---

# 34. Principio general del proyecto

Primero construir:

```text
MVP pequeño
    ↓
Funcional
    ↓
Testeado
    ↓
Demostrable
```

Después agregar:

```text
Inventario físico
Aprobaciones
Reportes
Auditoría
Gráficos
Excel / PDF
Compras avanzadas
Recetas
POS
```

La prioridad es evitar construir demasiadas funciones antes de validar que el núcleo del inventario funciona correctamente.

> Para este proyecto conviene terminar primero el MVP y posteriormente ampliar el sistema mediante SPECs independientes.
