# 重新设计配置加载逻辑，单容器模式沿用读写分离的配置参数

## 1. 需求分析

根据用户反馈，我们需要重新设计配置加载逻辑，让单容器模式的配置参数沿用读写分离的配置参数。具体需求：

1. 单容器模式的配置参数应该沿用读写分离的配置参数
2. 常规业务基本都应该是读写分离双容器，只有特殊情况下才是单容器管理
3. 变量名一致可以简化代码的关联性
4. 提高代码的可维护性
5. 无需修改代码，即可灵活切换不同的后端模式

## 2. 实现方案

### 2.1 统一配置参数命名

统一使用读写分离的配置参数命名，单容器模式下，读实例和写实例指向同一个URL：

* 写实例配置参数：

  * `CRUD_API_WRITE_INSTANCE_URL`：写实例URL

  * `CRUD_API_WRITE_INSTANCE_TIMEOUT`：写实例超时时间

  * `CRUD_API_WRITE_INSTANCE_RETRIES`：写实例重试次数

* 读实例配置参数：

  * `CRUD_API_READ_INSTANCE_URL`：读实例URL

  * `CRUD_API_READ_INSTANCE_TIMEOUT`：读实例超时时间

  * `CRUD_API_READ_INSTANCE_RETRIES`：读实例重试次数

* 后端模式配置参数：

  * `CRUD_API_BACKEND_TYPE`：后端类型（single、read\_write\_split、load\_balance）

### 2.2 根据后端类型动态配置实例

根据后端类型，动态配置实例列表：

* 单容器模式：读实例和写实例指向同一个URL

  * `CRUD_API_READ_INSTANCE_URL` 默认为 `CRUD_API_WRITE_INSTANCE_URL` 的值

  * 如果没有配置 `CRUD_API_WRITE_INSTANCE_URL`，则使用默认值 `http://localhost:8000`

* 读写分离模式：读实例和写实例指向不同的URL

  * 必须配置 `CRUD_API_WRITE_INSTANCE_URL` 和 `CRUD_API_READ_INSTANCE_URL`

* 负载均衡模式：支持多个读实例和写实例

  * 使用 `CRUD_API_INSTANCE_<INDEX>_<PROPERTY>` 格式的环境变量配置多个实例

### 2.3 简化调度逻辑

由于统一了配置参数命名，调度逻辑可以简化：

* 写操作：使用写实例

* 读操作：使用读实例

* 无需根据模式区分不同的调度逻辑

## 3. 实现步骤

1. **修改配置加载逻辑**

   * 更新 `src/config/mod.rs` 中的 `from_env` 方法

   * 统一使用读写分离的配置参数命名

   * 根据后端类型，动态配置实例列表

   * 单容器模式下，读实例和写实例指向同一个URL

2. **简化调度器逻辑**

   * 更新 `src/scheduler/mod.rs` 中的调度逻辑

   * 简化调度逻辑，无需根据模式区分不同的调度逻辑

   * 写操作使用写实例，读操作使用读实例

3. **测试配置加载**

   * 使用不同的配置参数，测试单容器模式

   * 使用不同的配置参数，测试读写分离模式

   * 使用不同的配置参数，测试负载均衡模式

   * 验证配置是否正确加载

4. **测试调度逻辑**

   * 启动应用，测试单容器模式

   * 启动应用，测试读写分离模式

   * 启动应用，测试负载均衡模式

   * 验证调度逻辑是否正确

## 4. 部署说明

部署时，可以通过以下环境变量配置不同类型的后端实例：

### 4.1 单容器模式

```bash
# 设置后端类型为单容器
CRUD_API_BACKEND_TYPE=single

# 配置单实例（读写实例指向同一个URL）
CRUD_API_WRITE_INSTANCE_URL=http://localhost:8000
```

### 4.2 读写分离模式

```bash
# 设置后端类型为读写分离
CRUD_API_BACKEND_TYPE=read_write_split

# 配置写实例
CRUD_API_WRITE_INSTANCE_URL=http://10.168.3.165:7981

# 配置读实例
CRUD_API_READ_INSTANCE_URL=http://10.168.3.168:7982
```

### 4.3 负载均衡模式

```bash
# 设置后端类型为负载均衡
CRUD_API_BACKEND_TYPE=load_balance

# 配置实例1（写实例）
CRUD_API_INSTANCE_0_ID=write-01
CRUD_API_INSTANCE_0_URL=http://10.168.3.165:7981
CRUD_API_INSTANCE_0_TYPE=write

# 配置实例2（读实例）
CRUD_API_INSTANCE_1_ID=read-01
CRUD_API_INSTANCE_1_URL=http://10.168.3.168:7982
CRUD_API_INSTANCE_1_TYPE=read
```

这样设计可以让单容器模式的配置参数沿用读写分离
