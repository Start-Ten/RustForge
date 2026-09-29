# STEP 2.5 — 接口冻结清单（唯一公开 API 来源）

状态标记：`冻结` = 签名不可改（变更需提案）；`可扩展` = 允许新增方法/变体（trait 与 enum 允许非破坏扩展）。
每个条目给出 crate、类型/签名。实现必须与本清单严格一致；新增公开项必须回填本清单（追加条目 + 变更提案记录）。

## 0. rf-core

| # | 接口 | 状态 |
|---|---|---|
| IF-001 | `pub const ENGINE_NAME: &str = "RustForge"` | 冻结 |
| IF-002 | `pub const ENGINE_VERSION: Version` | 冻结 |
| IF-003 | `pub struct Version { pub major: u32, pub minor: u32, pub patch: u32 }` + `new/parse?/display`，`Ord` | 冻结 |
| IF-004 | `pub struct Entity { pub index: u32, pub generation: u32 }`；`PLACEHOLDER`；`to_bits()->u64`；`from_bits(u64)->Option<Entity>` | 冻结 |
| IF-005 | `pub struct Tick(pub u32)`；`next(self)->Tick`（回绕安全） | 冻结 |
| IF-006 | `pub struct Handle<T> { pub index: u32, pub generation: u32 }`；`matches(&self, gen: u32)->bool` | 冻结 |
| IF-007 | `pub enum EngineError { EntityDead(Entity), ComponentNotRegistered(&'static str), StaleHandle, Unsupported(&'static str), NotYetSupported { what: &'static str, priority: &'static str }, Io(std::io::Error), InvalidData(String), Message(String) }`（thiserror） | 冻结 |
| IF-008 | `pub type Result<T, E = EngineError> = std::result::Result<T, E>` | 冻结 |
| IF-009 | `pub struct Rgba8Image { pub width: u32, pub height: u32, pub data: Vec<u8> }`；`new/filled/from_raw/with_mut/get/set/as_bytes/clear` | 冻结 |
| IF-010 | `pub fn fnv1a64(data: &[u8]) -> u64`；`pub fn fnv1a128(data: &[u8]) -> u128` | 冻结 |
| IF-011 | `pub struct Pcg32 { ... }`；`new(seed: u64)/next_u32/next_f32/range(i32,i32)/pick(&[T])->Option<&T>` | 冻结 |

## 1. rf-math

| # | 接口 | 状态 |
|---|---|---|
| IF-020 | `Vec2/Vec3/Vec4`：`new/splat/zero/one`、算术运算、`dot/cross(Vec3)/length/length_sq/normalize/normalized/lerp/distance/min/max(Clamp 分量)/sum`、`Vec2::from_angle/perp/rotate` | 冻结 |
| IF-021 | `Rot2 { pub cos: f32, pub sin: f32 }`：`identity/from_angle/angle/mul_rot/inverse/rotate(Vec2)` | 冻结 |
| IF-022 | `Mat2x3 { pub m: [f32; 6] }`（2D 仿射）：`identity/from_scale/from_translation/compose(scale,rot,trans)/mul_mat2x3/transform_point/transform_vector/inverse/to_scale_rot_trans` | 冻结 |
| IF-023 | `Mat4 { pub m: [f32; 16] }`（列主序）：`identity/from_translation/from_scale/from_axis_angle/from_quat/perspective/orthographic/look_at/mul_mat4/transform_point3/transform_vector4/transpose/inverse/translation()/apply_scale` | 冻结 |
| IF-024 | `Quat { pub x,y,z,w: f32 }`：`identity/from_axis_angle/from_rotation_x/y/z/mul_quat/normalize/normalized/slerp/nlerp/rotate_vec3/to_mat4` | 冻结 |
| IF-025 | `Transform { pub position: Vec3, pub rotation: Quat, pub scale: Vec3 }`：`identity/to_mat4/lerp`；`Transform2D { pub position: Vec2, pub rotation: Rot2, pub scale: Vec2 }`：`identity/to_mat2x3/lerp` | 冻结 |
| IF-026 | `AABB2/AABB3 { pub min, pub max }`：`from_center_half/from_points/contains/intersects/union/expand/center/size/is_valid` | 冻结 |
| IF-027 | `Circle { center: Vec2, radius }`、`Sphere { center: Vec3, radius }`：`contains/intersects` | 冻结 |
| IF-028 | `Plane { pub normal: Vec3, pub d: f32 }`：`from_normal_point/distance/sign`; `Ray2 { origin, dir }`/`Ray3`：`point_at(t)` | 冻结 |
| IF-029 | `Frustum { pub planes: [Plane; 6] }`：`from_view_projection(Mat4)/contains_point/intersects_aabb(&AABB3)->bool` | 冻结 |
| IF-030 | `Color { pub r,g,b,a: f32 }`：`rgb/rgba/hex? no/WHITE/BLACK/TRANSPARENT/to_rgba8/with_alpha/lerp` | 冻结 |
| IF-031 | 自由函数：`lerp_f32/smoothstep/clamp01/ease_in_out_cubic/angle_lerp` | 冻结 |
| IF-032 | `pub struct Curve1 { keys: Vec<CurveKey> }`；`CurveKey { t: f32, v: f32 }`；`sample(t)->f32`（线性+smoothstep 模式） | 冻结 |
| IF-033 | 相交：`ray_aabb2(&Ray2,&AABB2)->Option<(f32,f32)>`、`ray_sphere(&Ray3,&Sphere)->Option<f32>`、`ray_plane(&Ray3,&Plane)->Option<f32>`、`segment_intersect_2d` | 冻结 |
| IF-034 | 噪声：`value_noise_2d/perlin_2d/fbm_2d`（确定性，seed 参数） | 冻结 |

## 2. rf-memory

| # | 接口 | 状态 |
|---|---|---|
| IF-040 | `pub struct BumpArena`：`with_capacity(usize)/alloc<T>(&mut,T)->&mut T/alloc_slice<T>/reset/used/capacity` | 冻结 |
| IF-041 | `pub struct Pool<T>`（索引句柄）：`with_capacity/alloc(T)->u32/free(u32)->Option<T>/get/get_mut/len/is_empty/iter` | 冺结（笔误纠正：冻结） |
| IF-042 | `pub struct StackAllocator`：`with_capacity/alloc(usize,usize)->Option<NonNull<u8>>/mark()->usize/rewind(usize)/used` | 冻结 |
| IF-043 | `pub struct BuddyAllocator`：`new(total: usize, min_block: usize)/alloc(usize)->Option<NonNull<u8>>/free(NonNull<u8>)/stats` | 冻结 |
| IF-044 | `pub struct TrackingGlobal<A: GlobalAlloc>`：`new(inner)/set_enabled(bool)/capture_sites(bool)/snapshot()->AllocReport`；`AllocReport { live_bytes, live_count, peak_bytes, total_allocs, sites: Vec<AllocSite> }`；`AllocSite { backtrace: String, bytes: usize, count: usize }` | 冻结 |
| IF-045 | `pub fn leak_report(rep: &AllocReport) -> String` | 冻结 |

## 3. rf-event

| # | 接口 | 状态 |
|---|---|---|
| IF-050 | `pub trait Event: Send + Sync + 'static {}`；`pub enum Priority { Critical, High, Normal, Low }` | 冻结 |
| IF-051 | `pub struct EventBus`：`new/publish<E:Event>(&self,&E)/publish_deferred/flush_deferred(&mut)/subscribe<E,F:Fn(&E)+Send+Sync+'static>(F)->SubId/subscribe_once/unsubscribe(SubId)/set_rate_limit<E>(usize)`；`pub struct SubId(pub u64)` | 冻结 |
| IF-052 | `pub struct InputMap<B: Copy+Eq+Hash>`：`bind_action(name:&str,B)/bind_axis(name:&str,B,B,scale:f32)/poll(&mut,impl Fn(&B)->bool)->Vec<InputActionEvent>`；`InputActionEvent { name: String, value: f32, kind: ActionKind }`；`ActionKind { Pressed, Released, Held }` | 冻结 |

## 4. rf-task

| # | 接口 | 状态 |
|---|---|---|
| IF-060 | `pub enum JobPriority { Critical, High, Normal, Low, Background }` | 冻结 |
| IF-061 | `pub struct JobSystem`：`new(workers: usize)/num_workers/spawn<F:FnOnce()+Send+'static>(JobPriority,F)->JobHandle/spawn_after(&[JobHandle],JobPriority,F)->Result<JobHandle>/run_main_queue(&mut self)/spawn_main<F>/wait_all/shutdown` | 冻结 |
| IF-062 | `pub struct JobHandle`：`is_finished(&self)/wait(&self)` | 冻结 |
| IF-063 | `pub struct CancellationToken`：`new/cancel/is_cancelled/child` | 冻结 |
| IF-064 | `pub struct Latch`：`new(usize)/count_down/wait` | 冻结 |

## 5. rf-ecs

| # | 接口 | 状态 |
|---|---|---|
| IF-070 | `pub trait Component: Send + Sync + 'static {}`；`pub trait Resource: Send + Sync + 'static {}` | 冻结 |
| IF-071 | `pub struct World`：`new/spawn_empty/spawn<B:Bundle>(B)->Entity/spawn_batch/despawn(Entity)->bool/despawn_recursive/contains/entities()->Vec<Entity>/entity_count/clear` | 冻结 |
| IF-072 | World 组件：`insert<C:Component>(Entity,C)->Result/insert_if/remove<C>()->Option<C>/get<C>()->Option<&C>/get_mut/has<C>` | 冻结 |
| IF-073 | World 资源：`insert_resource<R:Resource>(R)/resource<R>()->Option<&R>/resource_mut/remove_resource` | 冻结 |
| IF-074 | World 查询：`query<D:QueryData,F:QueryFilter>(&mut self)->Query<'_,D,F>`（借出期间禁其余 &mut world —— 由 Rust 借用保证） | 冻结 |
| IF-075 | World 层级：`set_parent(Entity,Option<Entity>)->Result`（循环检测报错）；`pub struct Parent(pub Entity)`；`pub struct Children(pub Vec<Entity>)`；`children(Entity)->&[Entity]/parent(Entity)->Option<Entity>` | 冻结 |
| IF-076 | World 事件：`add_event<E:Send+Sync+'static>()/send<E>(E)/events<E>()->Option<&Events<E>>`；`pub struct Events<E>{..}`：`send/update/len/clear_current`；`pub struct EventReader<E>{..}`：`new/iter(&Events<E>)->impl Iterator<Item=&E>` | 冻结 |
| IF-077 | World tick：`change_tick(&self)->Tick/advance_tick()`；`pub struct WorldStats`：`world.stats()->WorldStats{archetypes,entities,columns:Vec<(String,usize)>}` | 冻结 |
| IF-078 | `pub unsafe trait QueryData { type Item<'a>; fn collect_access(&self? no: assoc fn) — 冻结签名：`unsafe fn fetch<'a>(arch: &'a Archetype, row: usize) -> Self::Item<'a>; fn access(acc: &mut AccessSet)` | 冻结 |
| IF-079 | QueryData 实现：`&T`、`&mut T`、`Option<&T>`、`Entity`、元组 (A..F ≤6) | 可扩展 |
| IF-080 | `pub trait QueryFilter { fn matches(arch: &ArchetypeInfo) -> bool }`；实现：`()`、`With<T>`、`Without<T>`、`Added<T>`、`Changed<T>` | 可扩展 |
| IF-081 | `pub struct Query<'w,D,F>`：`iter()->QueryIter/single()->Option/iter->count via iter().count()`；`QueryIter: Iterator<Item=D::Item<'_>>` | 冻结 |
| IF-082 | `pub struct AccessSet`：`read(TypeId)/write(TypeId)/reads()/writes()/conflicts(&AccessSet)->bool` | 冻结 |
| IF-083 | `pub trait Bundle { fn signatures(out:&mut Signatures); unsafe fn attach(self, arch:&mut Archetype, row:usize, tick: Tick) }`（元组 ≤6 实现） | 冻结 |
| IF-084 | `pub struct Commands`（命令缓冲）：`spawn(B)->()? 延迟—冻结签名：`fn queue(Command)`；`Command` enum { Spawn(B 的 Box 化经 TypeErasedBundle), Despawn(Entity), Insert(Entity, TypeErasedComponent), Remove(Entity, TypeId), SetParent(Entity, Option<Entity>) }；`apply(&mut World)`；对外 API：`commands.spawn(bundle)/despawn/insert/remove/set_parent` + `world.flush_commands(&mut Commands)` | 冻结 |
| IF-085 | `pub trait System: Send+Sync { fn name(&self)->&str; fn run(&mut self, world:&mut World); fn access(&self)->AccessSet }`；`pub fn system<F>(name:&str,F)->FnSystem<F>` | 冻结 |
| IF-086 | `pub struct SystemSet { name, parallel }`；`pub struct Stage { name, run_if }`；`pub struct Schedule`：`add_stage/add_system(stage,set?,Box<dyn System>)/run(&mut World, Option<&JobSystem>)->ScheduleReport`（冲突系统自动串行分组，非冲突并行） | 冻结 |

## 6. rf-reflection

| # | 接口 | 状态 |
|---|---|---|
| IF-090 | `pub struct FieldMeta { display_name, tooltip, category, range:Option<(f64,f64)>, step:Option<f64>, readonly, hidden, editor_exposed, script_exposed, network_replicated }`（全 pub + Default） | 冻结 |
| IF-091 | `pub enum FieldKind { F32,F64,Bool,I32,U32,U64,String,Vec2,Vec3,Vec4,Color,Entity,Enum(&'static [(&'static str,u64)]),Struct(&'static TypeDescriptor) }` | 可扩展 |
| IF-092 | `pub struct FieldInfo { name:&'static str, kind: FieldKind, offset: usize, meta: FieldMeta }`；`pub struct MethodInfo { name, args:&'static [&'static str], return_type:&'static str }` | 冻结 |
| IF-093 | `pub struct TypeDescriptor { type_path:&'static str, size:usize, align:usize, fields:&'static [FieldInfo], methods:&'static [MethodInfo] }` | 冻结 |
| IF-094 | `pub trait Reflect: Send+Sync+'static { fn descriptor()-> &'static TypeDescriptor where Self:Sized; fn as_reflect(&self)->&dyn Reflect; fn as_reflect_mut(&mut self)->&mut dyn Reflect }` | 冻结 |
| IF-095 | `pub enum FieldValue { F32(f32),F64,Bool,I32,U32,U64,String(String),Vec2(Vec2),Vec3,Vec4,Color(Color),Entity(Entity),Enum(u64),Opaque }` + From 转换 | 冻结 |
| IF-096 | `pub fn read_field(obj:&dyn Reflect, name:&str)->Result<FieldValue>`；`pub fn write_field(obj:&mut dyn Reflect, name:&str, FieldValue)->Result<()>`（经 offset 裸指针） | 冻结 |
| IF-097 | `pub struct TypeRegistry`：`register::<T:Reflect>()/register_descriptor(&'static TypeDescriptor)/get(type_path)->Option<&'static TypeDescriptor>/contains/count/paths()->Vec<&'static str>`；`global()->&'static Mutex<TypeRegistry>` | 冻结 |
| IF-098 | `macro_rules! reflect_struct`（生成 descriptor + Reflect impl；支持 `[range=(a..b)]`/`[readonly]`/`[hidden]`/`[category="..."]` 字段标注） | 可扩展 |

## 7. rf-serialization

| # | 接口 | 状态 |
|---|---|---|
| IF-100 | `pub enum SerFormat { Json, Binary }`；`pub struct SchemaVersion { major:u16, minor:u16, patch:u16 }`；`CURRENT_SCHEMA` | 冻结 |
| IF-101 | `pub fn to_json(&dyn Reflect)->Result<String>`；`pub fn from_json(&str,&mut dyn Reflect)->Result<()>`；`to_binary/from_binary`（字节稳定：字段按 descriptor 顺序） | 冻结 |
| IF-102 | `pub struct ComponentCodec { name:String, spawn: fn(&mut World, Entity) /* 构造默认组件并插入 */ }`；`pub struct SnapshotCodec { register::<C:Component+Reflect+Default>()/names() }` | 冻结 |
| IF-103 | `pub fn save_world(&World, codec:&SnapshotCodec, fmt)->Result<Vec<u8>>`；`pub fn load_world(&[u8], codec, &mut World)->Result<()>`（增量：`save_world_delta(&World, since: Tick, codec, fmt)`） | 冻结 |
| IF-104 | `pub struct Migration { from:(u16,u16), to:(u16,u16), renames:Vec<(String,String)>, drop:Vec<String>, defaults:Vec<(String, FieldValue)> }`；`pub fn migrate_json(doc:&mut serde_json::Value, chain:&[Migration])->Result<()>` | 冻结 |

## 8. rf-plugin

| # | 接口 | 状态 |
|---|---|---|
| IF-110 | `pub trait Plugin: Send+Sync { fn metadata(&self)->PluginMetadata; fn register(&self, &mut Registry)->rf_core::Result<()>; fn save_state(&self)->Option<String>{None} fn restore_state(&mut self, _:&str){} }` | 冻结 |
| IF-111 | `pub struct PluginMetadata { name, version: Version, author, description, license, dependencies: Vec<String> }` | 冻结 |
| IF-112 | `pub enum ExtensionPoint { RhiBackend, AssetImporter, EditorPanel, DebuggerProvider, ScriptHost, RenderPass, PhysicsBackend, AudioBackend, Renderer2DBackend, Physics2DBackend, Distribution }` | 可扩展 |
| IF-113 | `pub struct Registry { add(ExtensionPoint, Box<dyn Any+Send+Sync>)->SlotId/items(ExtensionPoint)->&[Box<dyn Any+Send+Sync>] }`；`SlotId(pub u64)` | 冻结 |
| IF-114 | `pub struct PluginHost { new()/load(Box<dyn Plugin>)->Result<String>/unload(name)->Result<Option<String>>/registry()->&mut Registry/loaded() }`（版本/依赖校验，状态 save/restore 往返） | 冻结 |
| IF-115 | feature `dynamic`：`pub fn load_library_plugin(path)->Result<Box<dyn Plugin>>`（libloading，符号 `rf_plugin_create`） | 冻结 |

## 9. rf-platform

| # | 接口 | 状态 |
|---|---|---|
| IF-120 | `pub struct WindowConfig { title,width,height,resizable,vsync,fullscreen,high_dpi,visible }`（Default：800×600） | 冻结 |
| IF-121 | `pub type WindowId = u64`；`pub enum WindowEvent { CloseRequested(WindowId),Resized(WindowId,u32,u32),Focused(WindowId,bool),Minimized(WindowId),ScaleChanged(WindowId,f32) }` | 可扩展 |
| IF-122 | `pub enum Key { A..Z,Num0..Num9,F1..F12,Escape,Enter,Space,Left,Right,Up,Down,ShiftLeft,ShiftRight,CtrlLeft,CtrlRight,AltLeft,AltRight,Tab,Backspace,Delete,Home,End,PageUp,PageDown,Comma,Period,Slash,Semicolon,Apostrophe,BracketLeft,BracketRight,Backslash,Minus,Equal,Backquote,CapsLock,Unknown }` | 可扩展 |
| IF-123 | `pub enum MouseButton { Left,Right,Middle,Other(u8) }`；`pub enum InputEvent { KeyPressed(Key),KeyReleased(Key),TextInput(char),MouseMoved{window,x,y},MouseWheel{dx,dy},MousePressed(MouseButton),MouseReleased(MouseButton),TouchBegin{id,x,y},TouchMove{id,x,y},TouchEnd{id,x,y},TouchCancel{id},GamepadConnected{id},GamepadDisconnected{id},GamepadButton{id,button:u8,pressed},GamepadAxis{id,axis:u8,value:f32} }` | 可扩展 |
| IF-124 | `pub enum PlatformEvent { Window(WindowEvent),Input(InputEvent),Suspended,Resumed,DestroyRequested,LowPower,Battery{level:Option<u8>,charging:bool} }` | 可扩展 |
| IF-125 | `pub trait Platform: Send { name()->&'static str; create_window(WindowConfig)->Result<WindowId>; window_size(WindowId)->(u32,u32); set_title(WindowId,&str); present(WindowId,&Rgba8Image)->Result<()>; poll_events()->Vec<PlatformEvent>; dpi_scale(WindowId)->f32; is_mobile()->bool{false}; request_exit() }` | 可扩展 |
| IF-126 | `pub enum PlatformKind { Default,Headless,Win32,X11,Wayland,Android }`；`pub fn create_platform(PlatformKind)->Result<Box<dyn Platform>>`（Win32 由 feature `win32` 门控，Windows 默认启用） | 冻结 |
| IF-127 | `pub trait FileSystem { read(&self,path)->Result<Vec<u8>>; exists(&self,path)->bool; list(&self,dir)->Vec<String> }`；`pub struct HostFs`；`pub struct OverlayFs` | 可扩展 |
| IF-128 | `pub struct FrameTimer { tick()->f32(dt), fps()->f32, frame_count()->u64 }`；`pub struct SystemInfo { os,arch,cores:u32,memory_mb:u64 }`；`pub fn system_info()->SystemInfo` | 冻结 |
| IF-129 | `pub trait Clipboard { set_text(&mut,&str); text(&mut)->Option<String> }`；`pub struct HeadlessClipboard`（内存往返） | 冻结 |

## 10. rf-rhi

| # | 接口 | 状态 |
|---|---|---|
| IF-140 | `pub enum Backend { Vulkan,Dx12,Dx11,OpenGL,OpenGLES,WebGPU,Metal,Software,Null }`；`display_name()->&'static str` | 冻结 |
| IF-141 | `pub struct Capabilities { max_texture_size,max_vertex_attributes:u32,max_color_attachments,supports_compute,supports_raytracing,supports_geometry_shader,supports_bc,supports_astc,uniform_buffer_alignment:usize,timestamp_period_ns:f32 }` | 可扩展 |
| IF-142 | `pub struct BackendInfo { name:String, backend:Backend, dedicated_gpu:bool, caps:Capabilities }` | 冻结 |
| IF-143 | `pub struct DeviceDesc { preferred:Backend, allow_fallback:bool, headless:bool, software_size:(u32,u32) }`；`pub fn create_device(DeviceDesc)->Result<(Box<dyn RhiDevice>,BackendInfo)>`（降级链 GPU→Software→Null） | 冻结 |
| IF-144 | `pub enum TextureFormat { Rgba8Unorm,Bgra8Unorm,R8Unorm,Rgba16Float,R32Float,Depth24Plus }`；`pub struct TextureUsage(pub u32)`：`SAMPLED|RENDER_TARGET|COPY_SRC|COPY_DST` 位运算 | 冻结 |
| IF-145 | `pub struct TextureDesc { width,height,format,usage,mip_levels,label }`；`pub struct BufferDesc { size:usize,usage,label }`；`BufferUsage(pub u32)`：`VERTEX|INDEX|UNIFORM|STORAGE|COPY_SRC|COPY_DST` | 冻结 |
| IF-146 | 句柄：`TextureHandle(pub u64)/BufferHandle(pub u64)/PipelineHandle(pub u64)`；`INVALID` 常量 | 冻结 |
| IF-147 | `pub enum ShaderModel { Flat,UnlitTextured,LambertTextured,Skybox? no—Wireframe }`；`pub struct PipelineDesc { model:ShaderModel, vertex_wgsl:Option<String>, fragment_wgsl:Option<String>, blend:BlendMode, depth_test:bool, depth_write:bool, cull:CullMode, label }`；`BlendMode { Opaque,Alpha,Additive,Multiply,Premultiplied }`；`CullMode { None,Back,Front }` | 可扩展 |
| IF-148 | `pub struct VertexLayout { stride:u32, attributes:Vec<VertexAttribute> }`；`VertexAttribute { name:&'static str, format:VertexFormat, offset:u32 }`；`VertexFormat { Float32x2,Float32x3,Float32x4,Uint8x4 }`；`IndexFormat { Uint16,Uint32 }` | 可扩展 |
| IF-149 | `pub trait RhiDevice: Send { backend_info()->&BackendInfo; create_texture(TextureDesc,Option<&[u8]>)->Result<TextureHandle>; update_texture(TextureHandle,u32,u32,x,y,w,h,&[u8])->Result<()>; read_texture(TextureHandle)->Result<Rgba8Image>; create_buffer(BufferDesc,Option<&[u8]>)->Result<BufferHandle>; update_buffer(BufferHandle,usize,&[u8])->Result<()>; create_pipeline(PipelineDesc,VertexLayout)->Result<PipelineHandle>; create_command_list()->Box<dyn CommandList>; submit(Box<dyn CommandList>)->Result<()>; begin_frame(Option<TextureHandle>)->Result<()>; end_frame()->Result<()>; set_present_target(Option<TextureHandle>); }` | 可扩展 |
| IF-150 | `pub trait CommandList { begin(); set_pipeline(PipelineHandle); set_texture(u32,TextureHandle); set_uniform_f32(&str,&[f32]); set_viewport(u32,u32,u32,u32); clear_color(Color)->Result; clear_depth(f32); set_vertex_buffer(BufferHandle); set_index_buffer(BufferHandle,IndexFormat); draw(u32,u32); draw_indexed(u32,u32); blit(TextureHandle,u32,u32,u32,u32,u32,u32); push_debug_group(&str); pop_debug_group(); end(); }` | 可扩展 |
| IF-151 | 内置实现：`SoftwareDevice`（CPU 光栅化，读写 Rgba8Image）、`NullDevice`；GPU 骨架 feature `gpu`：`create_device` 对 Vulkan/Dx12 返回 `Unsupported` 后按降级链处理 | 冻结 |

## 11. rf-asset

| # | 接口 | 状态 |
|---|---|---|
| IF-160 | `pub struct AssetId(pub u128)`；`from_path(&str)/from_bytes(&[u8])`；Display | 冻结 |
| IF-161 | `pub struct AssetHandle<T>{ id, generation:u32 }`；`pub enum AssetState { NotLoaded,Loading,Loaded,Failed(String) }` | 冻结 |
| IF-162 | 资产类型：`TextureAsset{image:Rgba8Image,srgb:bool,premultiplied:bool}`、`MeshAsset{positions,normals,uvs,indices,material}`、`SpriteSheetAsset{texture:AssetId,sprites:Vec<SpriteFrame>,animations:Vec<SpriteAnimation>}`、`TilemapAsset{width,height,tile_size,layers,properties}`、`AudioAsset{samples:Vec<f32>,channels,sample_rate}`、`FontAsset{image,glyphs:HashMap<char,GlyphRect>,size,line_height}`、`TextAsset(String)`、`DataAsset(serde_json::Value)`；均 `pub trait AssetData: Send+Sync+'static {}` | 可扩展 |
| IF-163 | `pub struct SpriteFrame { name,x,y,w,h,pivot:[f32;2] }`；`SpriteAnimation { name,frames:Vec<String>,fps,looping }`；`TileLayer { name,kind:TileLayerKind,data:Vec<u32>,width,height }`；`TileLayerKind { Tiles,Objects,Collision,Image }`；`GlyphRect { x,y,w,h,advance }` | 冻结 |
| IF-164 | `pub struct ImportedAsset { kind_name:&'static str, data:Box<dyn AssetData>, dependencies:Vec<AssetId>, warnings:Vec<String> }`；`pub struct ImportContext<'a> { pub source_path:&'a str, warnings:.., deps.. }`（`warn(&mut,&str)/add_dep(&mut,AssetId)`） | 冻结 |
| IF-165 | `pub trait AssetImporter: Send+Sync { fn name(&self)->&'static str; fn extensions(&self)->&'static [&'static str]; fn import(&self, ctx:&mut ImportContext, bytes:&[u8])->Result<ImportedAsset> }` | 冻结 |
| IF-166 | `pub struct ImportPipeline { new(cache_capacity:usize); register_importer(Box<dyn AssetImporter>); import_sync(&mut self, path)->Result<AssetId>; load_async(&mut self, &JobSystem, path, JobPriority)->AssetId; state(AssetId)->AssetState; get<T:AssetData>(AssetId)->Option<&T>; update(&mut self); notify_changed(&mut self,path)->bool(热重载,代数+1); build_index(&mut self, root:&str)->Result<usize>; loaded_count; warnings(AssetId)->&[String] }` | 冻结 |
| IF-167 | `pub struct LruCache { new(cap)/get/insert->Vec<evicted>/len/cap }`；`pub struct DependencyGraph { add_dep(AssetId,AssetId)/dependencies/dependents/has_cycle(topo 校验)/invalidate(AssetId)->Vec<AssetId>(受影响闭包,拓扑序) }` | 冻结 |
| IF-168 | `pub fn write_pak(path,&[(String,Vec<u8>)],compress:bool)->Result<u64>`；`pub struct PakReader{ open(path)/list()->Vec<String>/read(name)->Result<Vec<u8>>/verify(name)->bool }` | 冻结 |
| IF-169 | `pub trait AtlasPacker { fn pack(&self, &[(String,Rgba8Image)], padding:u32)->PackedAtlas }`；`PackedAtlas{ atlas:Rgba8Image, rects:HashMap<String,GlyphRect? no — Rect{name,x,y,w,h}->Vec<PackedRect> }`；`ShelfPacker` 实现 | 冻结 |
| IF-170 | 内置导入器（P0 可用）：`PngImporter/QoiImporter/BmpImporter/TgaImporter/PnmImporter/ObjImporter/GltfImporter/WavImporter/TmxImporter/JsonImporter/TomlImporter/CsvImporter/SpriteSheetJsonImporter/BitmapFontJsonImporter`；P1/P2 注册桩（返回 `NotYetSupported`）：`Jpg/Gif/WebP/Avif/Exr/Dds/Ktx2/Fbx/Usd/Blend/Stl/Ply/Video/Ttf/Otf` | 可扩展 |

## 12. rf-render

| # | 接口 | 状态 |
|---|---|---|
| IF-180 | `pub struct Camera2D { position:Vec2, rotation:f32, zoom:f32, viewport:(f32,f32), y_up:bool, pixel_perfect:bool }`：`view_matrix()->Mat2x3/world_to_screen/screen_to_world/set_ortho? 已含` | 冻结 |
| IF-181 | `pub struct Camera3D { position:Vec3, yaw,pitch,fov_y,near,far,ortho:bool,ortho_size }`：`view_matrix/projection_matrix(aspect)->Mat4/forward/right` | 冻结 |
| IF-182 | `pub struct SpriteInstance { transform:Mat2x3, uv:[f32;4], color:Color, sort_key:u32 }`；`pub struct SpriteBatchData { texture:Option<TextureHandle>, instances:Vec<SpriteInstance> }`；`pub fn build_batches(&mut Vec<SpriteBatchData>)->Vec<SpriteBatchData>`（按 sort_key+texture 稳定合并排序）；`DrawStats2D { draw_calls,sprites,texture_switches }` | 冻结 |
| IF-183 | `pub trait Renderer2D { begin_frame(&mut target:&mut Rgba8Image? — 冻结签名 begin_frame(&mut self, target: &mut Rgba8Image, camera:&Camera2D); draw_sprite(&mut self, tex:Option<&Rgba8Image>, uv:&[f32;4], color:Color, layer:i32, transform:&Mat2x3); flush(&mut self)->DrawStats2D }`；`SoftwareRenderer2D` 实现（含批次统计、像素完美、y 翻转） | 冻结 |
| IF-184 | `pub enum Light { Directional{dir,color,intensity},Point{pos,color,intensity,range},Spot{pos,dir,angle,color,intensity} }`；`pub enum Light2D { Point{pos,color,intensity,radius},Directional{dir,color,intensity},Ambient{color,intensity} }`；`pub struct MaterialParams { base_color,emissive,roughness,metallic }` | 可扩展 |
| IF-185 | `pub struct Renderer3D`：`render_mesh(&mut, target:&mut Rgba8Image, depth:&mut [f32], camera:&Camera3D, mesh:&MeshAsset, model:Mat4, material:&MaterialParams, texture:Option<&Rgba8Image>, lights:&[Light])`（CPU：变换→裁剪→z-buffer→Lambert+雾） | 冻结 |
| IF-186 | `pub trait RenderGraphPass { fn name(&self)->&str; fn kind(&self)->PassKind; fn inputs(&self)->Vec<ResourceId>; fn outputs(&self)->Vec<ResourceId>; fn execute(&mut self, ctx:&mut PassContext)->Result<()> }`；`PassKind{Raster,Compute,Copy,Present,TwoD}`；`ResourceId(pub u64)` | 可扩展 |
| IF-187 | `pub struct RenderGraph { declare_resource(name,desc)->ResourceId; add_pass(Box<dyn RenderGraphPass>)->PassId; compile()->Result<CompiledGraph>(拓扑排序+循环检测+BarrierPlan 列表); execute(&mut CompiledGraph, &mut PassContext? data)->Result<FrameStats> }`；`BarrierPlan{resource,state_from,state_to}`；`FrameStats{pass_times:Vec<(&'static str? String,u64)>}` | 冻结 |
| IF-188 | `pub struct LayerStack { add(name,LayerKind,order:i32)->LayerId; layers_in_order()->Vec<(&str,LayerKind)> }`；`LayerKind{TwoD,ThreeD}` | 冻结 |
| IF-189 | 后处理：`pub enum ToneMap{None,Reinhard,Aces,Filmic,AgX}` + `pub fn tone_map(&mut Rgba8Image,ToneMap)`；`bloom(&mut,f32,f32)`；`downsample(src,dst)`；`fxaa_lite(&mut)`；`pixel_perfect_scale(src, integer:bool)->Rgba8Image`；`crt_effect(&mut,f32)`；`chromatic_aberration(&mut,u32)`；`vignette(&mut,f32)` | 冻结 |
| IF-190 | `pub trait UpscalerProvider { name(&self)->&str; upscale(&self,&Rgba8Image,(u32,u32))->Result<Rgba8Image> }`；`BilinearUpscaler/Fsr1StyleUpscaler` 实现 | 可扩展 |
| IF-191 | Gizmo：`pub fn aabb2_hit_test(point,&AABB2)->Option<Handle2D>`；`Handle2D{Corner(i32? enum Corner::TL/TR/BL/BR),Body,Rotation}`；`pub fn draw_gizmo2d(&mut Rgba8Image,&AABB2,Color)`；3D：`pub fn axis_ray_hit(ray,&Mat4? — (ray, origin, scale)->Option<(u32 axis, f32 t)>` | 冻结 |

## 13. rf-physics

| # | 接口 | 状态 |
|---|---|---|
| IF-200 | `pub trait PhysicsWorld { step(&mut self,dt:f32); raycast(&self,origin:Vec3,dir:Vec3,max:f32)->Option<RaycastHit3> }`；`RaycastHit3{ point,normal,distance,body:usize }` | 冻结 |
| IF-201 | `pub enum Shape3 { Sphere(f32), Box(Vec3) }`；`pub struct Body3 { shape,position:Vec3,velocity,orientation:Quat,angular_velocity:Vec3,mass:Option<f32>(None=static),restitution,friction,linear_damping,sleeping,ccd:bool }` | 可扩展 |
| IF-202 | `pub struct PhysicsScene3d implements PhysicsWorld`：`new(gravity)/add_body(Body3)->usize/body_mut/remove_body/body_count/awake_all/set_gravity/overlap_sphere(Vec3,f32)->Vec<usize>`；确定性：固定步长 + `snapshot()->Vec<u8>`/`restore`、录制回放 `PhysicsRecorder{record/rewind? record(scene)->()/compare(scene)->bool}` | 冻结 |
| IF-203 | `pub enum Shape2 { Circle(f32), Box(Vec2), Polygon(Vec<Vec2>) }`；`pub struct Body2 { shape,position,velocity,rotation:f32,angular_velocity,mass:Option<f32>,restitution,friction,sleeping,one_way:bool }` | 可扩展 |
| IF-204 | `pub struct PhysicsScene2d`：`new(gravity)/add_body->usize/…/step(dt)/raycast(Ray2,f32)->Option<RaycastHit2>/add_joint(Joint2)/gravity`；`Joint2 { Distance{a,b,length},Revolute{a,b,anchor} }` | 冻结 |
| IF-205 | `pub struct CharacterController2D`：`move_body(&PhysicsScene2d, body:usize, displacement:Vec2)->MoveResult{ pos:Vec2, grounded:bool, hit_wall:bool }`（含单向平台、分离） | 冻结 |

## 14. rf-animation

| # | 接口 | 状态 |
|---|---|---|
| IF-210 | `pub struct Bone { name:String, parent:usize? i32, local_bind:Transform }`；`pub struct Skeleton { bones:Vec<Bone> }`：`global_binds()->Vec<Mat4>/inverse_binds()`；`pub struct Pose { locals:Vec<Transform> }`：`blend(&Pose,&Pose,f32)->Pose/identity(&Skeleton)`；`pub fn skinning_matrices(&Skeleton,&Pose)->Vec<Mat4>` | 冻结 |
| IF-211 | `pub struct Skeleton2 { bones:Vec<Bone2{name,parent:i32,local:Transform2D}> }`；`pub struct Pose2 { locals:Vec<Transform2D> }`；`pub fn bone_world_matrices(&Skeleton2,&Pose2)->Vec<Mat2x3>` | 冻结 |
| IF-212 | `pub struct FrameAnimation { frames:Vec<FrameStep{sprite:String,duration:f32,event:Option<String>}> }`；`pub struct AnimationPlayer2D { play(&FrameAnimation)/update(dt)->Option<PlaybackEvent{event,frame_index}>/current_sprite/progress }` | 冻结 |
| IF-213 | `pub struct StateMachine<S:Copy+Eq> { add_state/add_transition(from,to,f32)/set_condition_fn(Box<dyn Fn()->bool>)? — 冻结：`add_transition(from:S,to:S,fade:f32)`；`update(dt)->(S,S,f32)`（当前/前一/混合权重）；`force(S)` | 冻结 |
| IF-214 | `pub trait AnimationGraph { update(dt); }`——`pub struct ClipStateMachine<S>{...} implements`（clips + StateMachine 融合） | 可扩展 |
| IF-215 | IK：`pub fn two_bone_ik(root:Vec3,mid_len,end_len,target:Vec3,pole:Vec3)->(Vec3,Vec3)`；`pub fn fabrik(chain:&mut Vec<Vec3>,target:Vec3,iterations:usize,tol:f32)->bool`；`pub fn ik_2d(chain:&mut Vec<Vec2>,target,iterations,tol)->bool` | 冻结 |
| IF-216 | Sequencer-lite：`pub struct KeyTrack<T:Interp> { keys:Vec<(f32,T)> }`（`trait Interp { lerp(&self,&Self,f32)->Self }`）；`sample(f32)->Option<T>`；`pub struct Timeline { tracks, events:Vec<(f32,String)> }`；`pub struct TimelinePlayer { play/update(dt)->Vec<String>(fired) }` | 冻结 |

## 15. rf-audio

| # | 接口 | 状态 |
|---|---|---|
| IF-220 | `pub trait AudioGraph { render(&mut self, out:&mut [f32], rate:u32) }` | 冻结 |
| IF-221 | `pub enum AudioNode { Oscillator{kind:OscKind,freq,gain}, Sampler{samples:Arc<Vec<f32>>,pos:usize,rate_ratio,looping,gain}, Gain(f32), Pan(f32), Lowpass{cutoff}, Highpass{cutoff}, Envelope{attack,decay,sustain,release}, Delay{time_secs? samples:mix} }`；`OscKind{Sine,Square,Saw,Triangle,Noise}`（简化为值型节点，链式连接） | 可扩展 |
| IF-222 | `pub struct MixerGraph implements AudioGraph`：`new(rate)/add_node(AudioNode)->usize/connect(from,to)/set_param(node,param:&str,value)/render`；`pub fn render_to_wav(&MixerGraph, secs:f32, rate:u32)->Vec<u8>` | 冻结 |
| IF-223 | `pub fn spatialize(listener:Vec2, facing:f32, source:Vec2, max_dist:f32)->(f32,f32)`（距离衰减+声像） | 冻结 |

## 16. rf-network

| # | 接口 | 状态 |
|---|---|---|
| IF-230 | `pub trait NetworkTransport: Send { connect(addr:&str)->Result; send(&mut self, &[u8])->Result; recv(&mut self)->Vec<(String,Vec<u8>)>; connected()->bool; close() }` | 可扩展 |
| IF-231 | `pub struct LoopbackTransport`（成对 `loopback_pair(drop_rate:f32,latency_frames:u32)->(A,B)`） | 冻结 |
| IF-232 | `pub struct UdpTransport`（std::net，非阻塞） | 冻结 |
| IF-233 | 复制：`pub struct ReplicaStore { track(id:u64,priority:u8); mark_dirty(id,channel:u32); build_update(budget:usize, bytes_of: impl Fn)->? — 冻结：`build_snapshot(&mut, budget_bytes:usize, encode: &dyn Fn(u64,u32)->Vec<u8>)->Vec<u8>`（含序号头）；`apply_update(&mut, &[u8], decode:&dyn Fn(&mut World? no — 交付 Vec<(u64,u32,Vec<u8>)> 给上层) }`；`snapshot_all` | 冻结 |
| IF-234 | `pub struct RpcChannel { register(name,fn(&[u8])); call(name,&[u8],reliable:bool); dispatch_pending()->usize }`（可靠=重发队列+序号，不可靠=丢弃过期） | 冻结 |
| IF-235 | 预测回滚：`pub struct PredictionBuffer { push(tick:u64,input:Vec<u8>,state_hash:u64); reconcile(tick:u64, authoritative_hash:u64)->usize(回滚帧数) }` | 冻结 |

## 17. rf-ai

| # | 接口 | 状态 |
|---|---|---|
| IF-240 | `pub enum BtStatus { Success,Failure,Running }`；`pub trait BtNode { tick(&mut self, bb:&mut Blackboard, dt:f32)->BtStatus; name(&self)->&str }` | 冻结 |
| IF-241 | 组合节点：`Sequence/Selector/Parallel(success_needed:usize)/Invert/Cooldown(f32)/AlwaysSucceed` 构造器返回 `Box<dyn BtNode>`；叶子：`action(name,F:Fn(&mut Blackboard)->BtStatus)`、`condition(name,F:Fn(&Blackboard)->bool)` | 可扩展 |
| IF-242 | `pub enum BbValue { Bool(bool),F32(f32),I64(i64),Str(String),Vec2(Vec2) }`；`pub struct Blackboard { set/get_typed/keys/observe(name,cb:usize? id)->SubId? 简化：on_change(&mut,name,Box<dyn Fn()+Send+Sync>)->u64/remove_observer }` | 冻结 |
| IF-243 | `pub struct NavGrid { new(w,h)/blocked(x,y)->bool/set_blocked/in_bounds }`；`pub fn astar(&NavGrid, start:(i32,i32), goal:(i32,i32))->Option<Vec<(i32,i32)>>`（8 向、禁切角）；`pub fn flow_field(&NavGrid, goal)->Vec<u8>`（方向编码）；`pub fn follow(flow:&[u8], pos)->Option<(i32,i32)>` | 冻结 |
| IF-244 | `pub struct Perception2d { new(view_range:f32, fov_rad:f32, memory:f32)/sense(&self, now:f32, observer:Vec2, facing:Vec2, targets:&[(u64,Vec2)])->Vec<Perceived>`；`Perceived{id,last_seen:Vec2,visible:bool}` | 冻结 |
| IF-245 | 群体：`pub fn boids(self_pos,self_vel,neighbors:&[(Vec2,Vec2)], separation_w,alignment_w,cohesion_w)->Vec2` | 冻结 |

## 18. rf-ui

| # | 接口 | 状态 |
|---|---|---|
| IF-250 | `pub struct UiNode { id:u64, kind:WidgetKind, layout:Layout, style:Style, bind:Option<String>, children:Vec<UiNode>, rect:(f32,f32,f32,f32)(布局产物) }`；`WidgetKind{Container,Button,Label{text},Image{uv:[f32;4]},NineSlice{insets:(f32,f32,f32,f32)},Slider{min,max,value},Checkbox{checked},Panel,List{items:usize,item_h:f32,scroll:f32}}` | 可扩展 |
| IF-251 | `pub enum Layout { StackV{spacing},StackH{spacing},Grid{cols:usize},Overlay,Absolute }`；`pub struct Style { background:Option<Color>, border:Option<(f32,Color)>, text_color:Color, font_size:f32, corner:f32 }` | 可扩展 |
| IF-252 | `pub fn layout_tree(&mut UiNode, avail:(f32,f32))`（确定性布局）；`pub fn hit_test(&UiNode, x:f32,y:f32)->Option<u64>` | 冻结 |
| IF-253 | 显示列表：`pub enum UiCommand { Rect{x,y,w,h,color,corner}, NineSlice{x,y,w,h,uv,insets,color}, Text{x,y,text,size,color}, Line{x1,y1,x2,y2,color,thick} }`；`pub fn build_draw_list(&UiNode, out:&mut Vec<UiCommand>)` | 冻结 |
| IF-254 | `pub struct BindingStore { set(&mut,name,BbValue)/get/changed()->Vec<String> }`；`pub fn apply_bindings(&mut UiNode, &BindingStore)` | 冻结 |
| IF-255 | `pub fn list_visible_range(items:usize,item_h:f32,scroll:f32,viewport:f32)->(usize,usize)`（虚拟化窗口） | 冻结 |

## 19. rf-script

| # | 接口 | 状态 |
|---|---|---|
| IF-260 | `pub enum Value { Null,Bool(bool),F64(f64),I64(i64),Str(String) }`（算术/Display/From） | 冻结 |
| IF-261 | `pub trait ScriptHost { name(&self)->&str; compile(&mut self, src:&str)->Result<ScriptId>; call(&mut self, ScriptId, "main", &[Value])->Result<Value> }`；`ScriptId(pub u64)` | 冻结 |
| IF-262 | `pub struct BytecodeHost implements ScriptHost`：`set_global(name,Value)/register_native(name,arity,fn(&[Value])->Result<Value>)/globals()/disassemble(ScriptId)->String`；语言：表达式 + `let` + 赋值 + `if/else` + `while` + `fn? no` + `return` + native 调用 | 冻结 |
| IF-263 | VM 调试：`pub enum VmEvent { Breakpoint(ScriptId,u32), Step(ScriptId,u32), Trace(ScriptId,u32) }`；`pub struct ScriptDebugger { add_breakpoint(ScriptId,ip)/clear/step_mode(bool)/set_callback(Box<dyn FnMut(VmEvent)+Send>) }`；`BytecodeHost::attach_debugger(ScriptDebugger)/call_stepped(...)` | 冻结 |
| IF-264 | 可视化图：`pub struct GraphNode { id:u64, op:GraphOp, exec_in:bool }`；`GraphOp{Event(name),Branch,DoSequence,Print,SetVar,GetVar,Add,Sub,Mul,Lt,Gt,ConstF64}`；`pub struct GraphScript { nodes/edges:Vec<(u64,u32,u64,u32)> }`；`pub fn compile_graph(&GraphScript)->Result<ScriptId>`（编译进 BytecodeHost） | 可扩展 |
| IF-265 | GAS-lite：`pub struct AttributeSet { get/set/adjust }`；`pub struct GameEffect { attr:String, delta:Option<f64>, set_to:Option<f64>, duration:f32, period:Option<f32>, require_tags:Vec<String>, forbid_tags:Vec<String> }`；`pub struct Ability { name, cost:Vec<GameEffect>, cooldown:f32, tags:Vec<String> }`；`pub struct AbilitySystem { add_attribute/add_effect(->id)/try_use_ability(name)->Result<bool>/tick(dt)->Vec<String>(到期事件)/has_tag/add_tag/remove_tag/cooldown_remaining }` | 冻结 |

## 20. rf-ml

| # | 接口 | 状态 |
|---|---|---|
| IF-270 | `pub struct Tensor { shape:Vec<usize>, data:Vec<f32> }`：`zeros/ones/from_vec(shape,data)/matmul/element_add/element_mul? scale/transposed/argmax/mse/rmse` | 冻结 |
| IF-271 | `pub trait MlBackend { name; train(&mut self, samples:&[(Tensor,Tensor)], epochs, lr)->TrainingReport; infer(&mut,&Tensor)->Result<Tensor>; save(&self,path)->Result; load(&mut,path)->Result }`；`TrainingReport { epochs, final_loss, history:Vec<f32> }` | 冻结 |
| IF-272 | `pub enum Activation { Identity,ReLU,Sigmoid,Tanh }`（fn/derivative） | 冻结 |
| IF-273 | `pub struct MlpBackend implements MlBackend`（Dense 层 + SGD + MSE；纯 Rust，桌面训练/各端推理） | 冻结 |

## 21. rf-editor

| # | 接口 | 状态 |
|---|---|---|
| IF-280 | `pub enum EditorMode { TwoD,ThreeD,Mixed }`；`pub enum PanelKind { Viewport,Hierarchy,Properties,AssetBrowser,Console,Profiler,CommandPalette,Custom(&'static str) }` | 可扩展 |
| IF-281 | `pub struct Selection { entities:Vec<Entity>, assets:Vec<AssetId> }`：`toggle_entity/is_entity_selected/select_one/clear` | 冻结 |
| IF-282 | `pub struct EditorContext<'a> { pub world:&'a mut World, pub mode:&'a mut EditorMode, pub selection:&'a mut Selection, pub undo:&'a mut UndoStack, pub log:&'a mut Vec<String> }` | 冻结 |
| IF-283 | `pub trait EditorPanel: Send { kind(&self)->PanelKind; title(&self)->String; update(&mut self, ctx:&mut EditorContext)->Result<()> }` | 可扩展 |
| IF-284 | `pub struct UndoStack { push(label:&'static str, do_fn,undo_fn)/undo(&mut,&mut World)->Option<&'static str>/redo/depth/can_undo/can_redo/clear }`（`Box<dyn Fn(&mut World)+Send>` 对） | 冻结 |
| IF-285 | `pub struct CommandPalette { register(name,shortcut,fn(&mut EditorContext))->? 冻结：`register(name:&str,shortcut:Option<&str>,Box<dyn Fn(&mut EditorContext)>)/search(query)->Vec<String>(模糊排序)/execute(name,ctx)->bool` | 冻结 |
| IF-286 | `pub struct Theme { name, colors:HashMap<String,Color> }`：`dark()/light()/color(name,Color)` | 冻结 |
| IF-287 | `pub struct DockLayout { set_columns(Vec<Vec<PanelKind>>)/columns()/to_json/from_json(corrupted->default)/reset }` | 冻结 |
| IF-288 | `pub struct EditorModel { mode,selection,undo,palette,theme,layout,dirty,autosave_path:Option<String> }`：`new(mode)/update(panels:&mut [Box<dyn EditorPanel>], world)->Result/save_state/load_state/mark_dirty` | 冻结 |
| IF-289 | 内置面板：`HierarchyPanel/PropertiesPanel/AssetBrowserPanel/ConsolePanel/ProfilerPanel/ViewportPanel`（实现 IF-283；Properties 由反射网格 `pub fn build_property_grid(&dyn Reflect)->Vec<PropertyRow{label,value:FieldValue,meta}>` 驱动） | 可扩展 |
| IF-290 | `pub struct EditorApp`：`new(WindowConfig, backend:Backend)->Result`（desktop）/`run()->Result`/`run_frame(events:&[PlatformEvent])->Result<FrameOutcome>`（headless 测试）/`model()->&EditorModel` | 冻结 |

## 22. rf-debugger

| # | 接口 | 状态 |
|---|---|---|
| IF-300 | `pub enum LogLevel { Trace,Debug,Info,Warn,Error }`：`as_str/parse`；`pub struct LogEntry { time_ms:u64, level, category:String, message:String }` | 冻结 |
| IF-301 | `pub struct Logger { set_level/add_sink(Box<dyn Fn(&LogEntry)+Send+Sync>)/log(level,category,message)/entries()->&VecDeque<LogEntry>/filter(level,category_contains)->Vec<LogEntry>/set_ring_capacity }`；全局 `pub fn init_global()/global()->&'static Mutex<Logger>`；宏 `rf_info!/rf_warn!/rf_error!/rf_debug!(category, "fmt", ..)` | 冻结 |
| IF-302 | `pub struct CVar { value:CVarValue, default:CVarValue, desc }`；`CVarValue{Bool/F32/I64/Str}`；`pub struct Console { register_cvar(name,desc,default)/cvar(name)->Option<&CVar>/set_cvar(name,CVarValue)->Result/register_command(name,help,Box<dyn Fn(&[&str])->String>)/execute(line:&str)->Result<String>/autocomplete(prefix)->Vec<String>/history }` | 冻结 |
| IF-303 | `pub struct Profiler { scope(name:&'static str)->ScopeGuard（Drop 记录）/counter(name,&'static str? -> 冻结：counter_add(name,f64)/counter_set(name,f64)/counters()->Vec<(String,f64)>/frame_start/frame_end(dt:f32)/fps()/frame_times()->&VecDeque<f32>/export_text()->String/flame_events()->Vec<FlameEvent> }`；`FlameEvent{name,start_ns:u64,dur_ns:u64,depth:u32}` | 冻结 |
| IF-304 | `pub trait DebugTransport { send(&mut self,&[u8])->Result; recv(&mut self)->Vec<Vec<u8>> }`；`pub struct RemoteDebugServer { new(transport)/poll_request_and_respond(&Console,&Profiler) }`（JSON 行协议：`stats`/`exec`） | 冻结 |
| IF-305 | 渲染统计：`pub struct RenderStats { sprites,draw_calls_2d,draw_calls_3d? batches,texture_switches,triangles,passes }`（由 renderer 填充、Profiler 展示） | 冻结 |

## 23. rf-distribute

| # | 接口 | 状态 |
|---|---|---|
| IF-310 | `pub enum BuildProfile { Debug,Development,Release,Shipping,Test }`：`cargo_profile()->&'static str/strip/symbols/debug_assertions/opt_level` | 冻结 |
| IF-311 | `pub enum PlatformTarget { WindowsX64,WindowsArm64,LinuxX64,LinuxArm64,AndroidArm64,AndroidArmv7,MacosArm64,MacosX64,IosArm64,WebWasm }`：`triple()->&'static str/is_host/is_desktop/is_mobile/needs_cross_tool()` | 冻结 |
| IF-312 | `pub struct BuildPlan { profile,target,features:Vec<String>,examples:Vec<String>,out_dir }`：`plan(...)/cargo_command_line()->Vec<String>/execute()->Result<BuildReport{duration_secs,success,artifacts:Vec<String>}>`（std::process 调 cargo） | 冻结 |
| IF-313 | `pub trait Packager { name; package(staging:&[StagedFile{path,bytes? path:PathBuf + src}... 冻结：`package(&self, out_dir:&Path, files:&[(String, Vec<u8>)])->Result<OutputArtifact>` }`；实现：`ZipPackager`（store+deflate）、`PakPackager`、`ScriptPackager{kind:ScriptKind}` 生成 InnoSetup/Nsis/AppImage 脚本/deb control/Gradle 工程；`OutputArtifact{ path:String, bytes:u64, crc32:u32, sha256:String }` | 可扩展 |
| IF-314 | `pub fn crc32(&[u8])->u32`；`pub fn sha256_hex(&[u8])->String`（自实现） | 冻结 |
| IF-315 | `pub trait CodeSigner { sign(&self, artifact:&Path, key:&HmacKey)->Result<SignatureInfo>; verify(&self, artifact:&Path, sig:&SignatureInfo, key:&HmacKey)->Result<bool> }`；`HmacKey::from_hex/from_env(new? from_bytes)`；`SignatureInfo{ sha256, hmac_sha256, signed_at_ms }`；`LocalHmacSigner` + `NativeTools::signing_command(platform,artifact)->String`（signtool/codesign/apksigner 命令生成） | 冻结 |
| IF-316 | `pub trait StoreUploader { name; prepare_manifest(&PublishBundle)->Result<String(json)>; upload(&self, manifest:&str, dry_run:bool)->Result<UploadReport> }`；实现：`SteamUploader/PlayStoreUploader/ItchUploader/GogUploader`（清单生成 + 命令/环境变量说明，dry-run 报告）；`UploadReport{ store, ok, detail }`；`PublishBundle{ app_name, version:Version, artifacts:Vec<OutputArtifact>, stores:Vec<String>, privacy_policy:bool, age_rating:Option<String> }` | 可扩展 |
| IF-317 | `pub struct UpdateManifest { version:Version, files:Vec<UpdateFile{path,size,sha256,blocks:Vec<String>(块哈希)}>, full_url }`：`build_manifest(dir,block:usize)->Result`/`parse`；`pub fn block_diff(old,new,block)->Vec<PatchOp{Copy{start,len},Data(Vec<u8>)}>` + `apply_patch(old,&[PatchOp])->Vec<u8>`；`pub struct Updater { check(current,&UpdateManifest)->Option<UpdatePlan>/apply_plan(dir,&UpdatePlan,fetch:&dyn Fn(&str)->Vec<u8>)->Result/回滚 backup }` | 冻结 |
| IF-318 | `pub trait AchievementProvider { unlock(id:&str,progress:f32)->Result<bool>; state(id:&str)->Option<(f32,bool)>; drain_notifications()->Vec<String> }`；`LocalAchievements{dir}`（JSON 持久化） | 冻结 |
| IF-319 | `pub trait CloudSaveProvider { upload(slot:&str,&[u8])->Result; download(slot:&str)->Option<Vec<u8>>; resolve(local:&[u8],remote:&[u8],ConflictPolicy)->ConflictResolution }`；`ConflictPolicy{PreferNewer,PreferLocal,PreferRemote,Manual}`；`LocalCloudSaves{dir}`（mtime 冲突检测） | 冻结 |
| IF-320 | `pub trait CrashReporter { capture(message:&str, backtrace:&str)->CrashReport; persist(&CrashReport)->Result<String(path)>; flush_queue()->Result<usize> }`；`CrashReport{ app_version,os,time_ms,message,backtrace }`；`LocalCrashReporter{dir,consent:bool}`（GDPR 同意门控 + 队列重试） | 冻结 |
| IF-321 | `pub trait AnalyticsProvider { track(event:&str, props:&[(String,String)]); flush()->Result<usize> }`；`LocalAnalytics{dir}`（jsonl 批处理） | 冻结 |
| IF-322 | `pub struct ComplianceChecker { add_rule(ComplianceRule)/run(&PublishBundle)->ComplianceReport }`；`ComplianceRule{ jurisdiction, requirement, check:Box<dyn Fn(&PublishBundle)->bool> }`；`ComplianceReport{ passed:Vec<String>, failed:Vec<String>, warnings:Vec<String> }`；内置规则集 `builtin_rules()`（GDPR/CCPA/隐私政策/年龄分级/中国版号提示） | 冻结 |
| IF-323 | `pub trait DrmProvider { name; wrap(artifact:&Path)->Result<String(说明或包装产物路径)> }`；`NoDrm/SteamDrmCommand`（命令生成，不内置）；`pub trait AntiCheatProvider { name; scan(dir:&Path)->AntiCheatReport{ clean:bool, findings:Vec<String> } }`；`IntegrityAntiCheat`（文件哈希校验） | 冻结 |
| IF-324 | `pub trait SdkExporter { export(out_dir:&Path)->Result<Vec<String>> }`；`CSdkExporter`（生成 rustforge.h C 声明 + README）；FFI：`pub mod ffi { rf_engine_version()->*const c_char / rf_sha256_hex(data,len,buf) / rf_init()->i32 }`（curated 集） | 冻结 |

## 24. rustforge（伞）

| # | 接口 | 状态 |
|---|---|---|
| IF-330 | `pub mod prelude`（全 crate 常用 re-export） | 可扩展 |
| IF-331 | `pub trait App { setup(&mut World, &mut ImportPipeline); update(&mut World, dt:f32); render_frame(&mut self? — 冻结：`fn render(&mut self, frame:&mut FrameCtx)` }`；`FrameCtx{ target:&mut Rgba8Image, camera2d:&mut Camera2D? — 组合：FrameCtx { image:&mut Rgba8Image } }` | 冻结 |
| IF-332 | `pub struct EngineConfig { window:WindowConfig, backend:Option<Backend>, headless:bool, target_fps:Option<f32>, frames:Option<u64>(headless 帧数) }`（Default 800×600 Software） | 冻结 |
| IF-333 | `pub struct Engine { new(EngineConfig)->Result; world()->&mut World? — &World + &mut World 方法; pipeline()->&mut ImportPipeline; device... run(app:Box<dyn App>)->Result; run_headless(app,frames:u64)->Result<RunSummary{frames,avg_fps}> }`；组装：Platform+Device+World+Schedule+Logger | 冻结 |

## 25. 依赖方向（冻结）

```
rf-core ← (全部)
rf-math → rf-core
rf-memory → rf-core
rf-event → rf-core
rf-task → rf-core
rf-ecs → rf-core, rf-math, rf-task
rf-reflection → rf-core, rf-math
rf-serialization → rf-core, rf-math, rf-reflection, rf-ecs
rf-plugin → rf-core
rf-platform → rf-core
rf-rhi → rf-core, rf-math
rf-asset → rf-core, rf-math, rf-reflection, rf-task, rf-serialization
rf-render → rf-core, rf-math, rf-ecs, rf-rhi, rf-asset
rf-physics → rf-core, rf-math, rf-ecs
rf-animation → rf-core, rf-math, rf-ecs
rf-audio → rf-core, rf-math, rf-asset(仅类型? no—不依赖，AudioAsset 移至 rf-audio? 决议：WavImporter 在 rf-asset 产出 rf-audio 定义的 AudioAsset → rf-asset → rf-audio 不行（asset 在 audio 上层）。决议：AudioAsset 定义于 rf-audio，rf-asset 依赖 rf-audio。)
  修正：rf-audio → rf-core, rf-math；rf-asset → + rf-audio
rf-network → rf-core, rf-serialization
rf-ai → rf-core, rf-math, rf-ecs
rf-ui → rf-core, rf-math
rf-script → rf-core, rf-ecs, rf-reflection
rf-ml → rf-core, rf-math
rf-editor → rf-core, rf-math, rf-ecs, rf-asset, rf-render, rf-ui, rf-debugger, rf-platform, rf-rhi, rf-reflection, rf-animation? (不需要—不依赖)
rf-debugger → rf-core
rf-distribute → rf-core, rf-asset(PAK)
rustforge → 全部
```
禁止：基础层→图形层；rf-rhi→rf-render；rf-ui→rf-render；rf-asset→rf-render。

## 26. 兼容性规则

- trait 与 enum 允许非破坏性扩展（新增方法需带默认实现）；struct 字段与函数签名冻结。
- 废弃流程：`#[deprecated]` + CHANGELOG 记录 → 保留一个次要版本 → 移除并升 minor。
- 公开 API 之外一律私有；`pub use` 仅发生在伞 crate 与 crate 根。

## 26.5 变更提案记录（实现期）

| CP | 接口 | 变更 | 原因 | 兼容性 |
|---|---|---|---|---|
| CP-001 | IF-029 Frustum | 新增私有字段 view_projection/perspective（公开字段不变） | 背面点经 6 平面测试假阳性（透视除法 w<0 陷阱） | 无公开 API 影响 |
| CP-002 | IF-150 CommandList | 新增 `as_any(&mut self)`（返回 &mut dyn Any） | Software 后端 submit 需下转取回录制命令 | 新增必需方法；实现方在仓内同步 |
| CP-003 | IF-303 Profiler::scope | 改为自由函数 `scope(name)`（线程本地） | &mut self 守卫无法嵌套作用域 | 语义等价，嵌套支持 |

## 27. 接口清单索引（统计）

条目总数：IF-001 ~ IF-334，共 334 内 230+ 条（编号有保留段）。后续新增以 IF-4xx 起编号并附变更提案。
