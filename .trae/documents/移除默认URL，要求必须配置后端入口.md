# 移除默认URL，要求必须配置后端入口

## 1. 需求分析

根据用户反馈，我们需要修改配置加载逻辑，移除默认URL，确保：

1. 项目不应存在默认URL
2. 当拉起容器时，若未配置后端入口，应抛出异常要求配置入口
3. 仅当配置了后端入口但访问不到时，才应拉取应急单容器解决后端离线问题

## 2. 实现方案

### 2.1 修改配置加载逻辑

更新 `from_env` 方法，移除默认URL，要求必须通过环境变量配置后端入口：

* 移除 `CRUD_API_WRITE_INSTANCE_URL` 的默认值，要求此环境变量必须设置
* 若未设置 `CRUD_API_WRITE_INSTANCE_URL`，`from_env` 方法返回错误，导致容器启动失败
* 保留 `CRUD_API_READ_INSTANCE_URL` 的默认值为 `write_instance_url.clone()`，允许只配置一个URL
* 保留 `CRUD_API_BACKEND_TYPE` 的默认值为 `read_write_split`，允许默认使用读写分离模式

### 2.2 配置验证

确保配置验证逻辑正确，只有当后端入口URL被正确配置时，容器才能启动成功。

## 3. 实现步骤

1. **修改配置加载逻辑**
   * 更新 `src/config/mod.rs` 中的 `from_env` 方法
   * 移除 `CRUD_API_WRITE_INSTANCE_URL` 的默认值，使用 `env::var` 直接返回结果，若未设置则抛出错误
   * 保留其他参数的默认值不变

2. **测试配置加载**
   * 测试未设置 `CRUD_API_WRITE_INSTANCE_URL` 时，容器启动失败，抛出异常
   * 测试设置 `CRUD_API_WRITE_INSTANCE_URL` 时，容器正常启动
   * 测试不同后端模式下的配置加载

3. **验证应急机制**
   * 确保当配置了后端入口但访问不到时，系统会拉取应急单容器

## 4. 部署说明

部署时，必须通过环境变量配置后端入口：

```bash
# 必须配置写实例URL
CRUD_API_WRITE_INSTANCE_URL=http://10.168.3.165:7981

# 可选配置读实例URL，默认与写实例相同
CRUD_API_READ_INSTANCE_URL=http://10.168.3.168:7982

# 可选配置后端类型，默认读写分离
CRUD_API_BACKEND_TYPE=read_write_split
```

这样，当未配置后端入口时，容器会抛出异常，要求必须配置，确保部署时正确设置后端地址。