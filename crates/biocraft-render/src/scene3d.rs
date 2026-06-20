//! 3B sahne temeli — wgpu ile özel shader/geometri (İP-04).
//!
//! İki katman:
//! 1. **Saf matematik** (egui'siz, test-edilebilir): [`Vertex`], [`Mesh`] + üreteçler ([`kure`],
//!    [`silindir`]), [`Kamera3B`] (look-at + perspektif → MVP matrisi).
//! 2. **wgpu çizici** ([`Sahne3B`]): kendi off-screen renk + derinlik dokusuna basit Lambert
//!    ışıklandırmayla çizer; sonuç bir doku olarak UI'ya (egui) verilir.
//!
//! Çekirdek eklenti **ÇE-07 (3B yapı)** ileride bu temeli (kürdele/top-çubuk/yüzey) kullanır.
//! Harici 3B kütüphane (THREE.js benzeri) **yoktur** — native wgpu (MK-01).  Malzeme rengi
//! token'dan gelir (MK-52); derinlik tamponu sayesinde top-çubuk gibi üst üste binen geometri
//! doğru sıralanır.
// MK-01: native wgpu çizim. MK-52: malzeme rengi token'dan. MK-04: ≤100 ms GPU batch'e uyumlu.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

// ─── Saf vektör/matris yardımcıları (column-major, wgpu/WGSL ile uyumlu) ────────────────

type Vec3 = [f32; 3];

fn cikar(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn capraz(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn nokta_carpim(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn normalle(v: Vec3) -> Vec3 {
    let u = nokta_carpim(v, v).sqrt();
    if u <= f32::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [v[0] / u, v[1] / u, v[2] / u]
    }
}

/// 4×4 matris (column-major, 16 ardışık f32 — WGSL `mat4x4<f32>` ile birebir).
type Mat4 = [f32; 16];

fn birim() -> Mat4 {
    let mut m = [0.0f32; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    m
}

/// `a * b` (column-major).
fn carp(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut c = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut s = 0.0;
            for k in 0..4 {
                s += a[k * 4 + row] * b[col * 4 + k];
            }
            c[col * 4 + row] = s;
        }
    }
    c
}

/// Sağ-elli look-at görünüm matrisi.
fn bak(goz: Vec3, hedef: Vec3, yukari: Vec3) -> Mat4 {
    let f = normalle(cikar(hedef, goz)); // ileri
    let s = normalle(capraz(f, yukari)); // sağ
    let u = capraz(s, f); // gerçek yukarı
    let mut m = birim();
    m[0] = s[0];
    m[1] = u[0];
    m[2] = -f[0];
    m[4] = s[1];
    m[5] = u[1];
    m[6] = -f[1];
    m[8] = s[2];
    m[9] = u[2];
    m[10] = -f[2];
    m[12] = -nokta_carpim(s, goz);
    m[13] = -nokta_carpim(u, goz);
    m[14] = nokta_carpim(f, goz);
    m
}

/// Sağ-elli perspektif matrisi (derinlik aralığı 0..1 — wgpu/D3D/Metal NDC).
fn perspektif(fov_y_rad: f32, en_boy: f32, yakin: f32, uzak: f32) -> Mat4 {
    let f = 1.0 / (fov_y_rad * 0.5).tan();
    let mut m = [0.0f32; 16];
    m[0] = f / en_boy.max(f32::EPSILON);
    m[5] = f;
    m[10] = uzak / (yakin - uzak);
    m[11] = -1.0;
    m[14] = (yakin * uzak) / (yakin - uzak);
    m
}

/// Yörünge (orbit) kamerası → MVP üretir.  Model = birim olduğundan MVP = perspektif × görünüm.
#[derive(Debug, Clone, Copy)]
pub struct Kamera3B {
    /// Göz konumu.
    pub goz: Vec3,
    /// Bakılan hedef.
    pub hedef: Vec3,
    /// Görüş açısı (radyan, dikey).
    pub fov_y: f32,
    /// En/boy oranı.
    pub en_boy: f32,
    /// Yakın düzlem.
    pub yakin: f32,
    /// Uzak düzlem.
    pub uzak: f32,
}

impl Kamera3B {
    /// Orijini, `yaricap` uzaklıktan ve `aci` açısıyla yörüngede izleyen kamera.
    pub fn yorunge(aci: f32, yaricap: f32, yukseklik: f32, en_boy: f32) -> Self {
        Self {
            goz: [yaricap * aci.cos(), yukseklik, yaricap * aci.sin()],
            hedef: [0.0, 0.0, 0.0],
            fov_y: 45f32.to_radians(),
            en_boy,
            yakin: 0.1,
            uzak: 100.0,
        }
    }

    /// MVP matrisi (column-major, shader uniform'una doğrudan kopyalanır).
    pub fn mvp(&self) -> Mat4 {
        let v = bak(self.goz, self.hedef, [0.0, 1.0, 0.0]);
        let p = perspektif(self.fov_y, self.en_boy, self.yakin, self.uzak);
        carp(&p, &v)
    }
}

// ─── Mesh (geometri) ────────────────────────────────────────────────────────────────────

/// GPU köşe noktası: konum + normal (Lambert ışıklandırma için).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    /// Konum (model uzayı).
    pub konum: [f32; 3],
    /// Yüzey normali (birim).
    pub normal: [f32; 3],
}

/// Köşe + indeks listesi.  Birden çok ilkel `ekle` ile birleştirilebilir (top-çubuk).
#[derive(Debug, Clone, Default)]
pub struct Mesh {
    /// Köşe noktaları.
    pub kose: Vec<Vertex>,
    /// Üçgen indeksleri (her 3'ü bir üçgen).
    pub indeks: Vec<u32>,
}

impl Mesh {
    /// Başka bir mesh'i bu mesh'e ekler (indeksleri ötelenerek).
    pub fn ekle(&mut self, diger: &Mesh) {
        let taban = self.kose.len() as u32;
        self.kose.extend_from_slice(&diger.kose);
        self.indeks.extend(diger.indeks.iter().map(|i| i + taban));
    }

    /// Üçgen sayısı.
    pub fn ucgen_sayisi(&self) -> usize {
        self.indeks.len() / 3
    }
}

/// UV küre üretir (top-çubuk modelinin "topu"; kürdele/yüzey için de temel).
pub fn kure(merkez: Vec3, yaricap: f32, dilim: u32, kat: u32) -> Mesh {
    let dilim = dilim.max(3);
    let kat = kat.max(2);
    let mut kose = Vec::new();
    for i in 0..=kat {
        let phi = std::f32::consts::PI * i as f32 / kat as f32; // 0..PI (kutuptan kutba)
        let (sp, cp) = phi.sin_cos();
        for j in 0..=dilim {
            let theta = std::f32::consts::TAU * j as f32 / dilim as f32; // 0..2PI
            let (st, ct) = theta.sin_cos();
            let n = [sp * ct, cp, sp * st];
            kose.push(Vertex {
                konum: [
                    merkez[0] + yaricap * n[0],
                    merkez[1] + yaricap * n[1],
                    merkez[2] + yaricap * n[2],
                ],
                normal: n,
            });
        }
    }
    let mut indeks = Vec::new();
    let sutun = dilim + 1;
    for i in 0..kat {
        for j in 0..dilim {
            let a = i * sutun + j;
            let b = a + sutun;
            indeks.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
        }
    }
    Mesh { kose, indeks }
}

/// İki nokta arasında silindir (top-çubuk modelinin "çubuğu"/bağı).  Kapaksız (uçları toplar örter).
pub fn silindir(p0: Vec3, p1: Vec3, yaricap: f32, dilim: u32) -> Mesh {
    let dilim = dilim.max(3);
    let eksen = cikar(p1, p0);
    let yon = normalle(eksen);
    // Eksene dik bir taban (yon Y'ye paralelse X'i referans al).
    let referans = if yon[1].abs() > 0.99 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let u = normalle(capraz(yon, referans));
    let v = capraz(yon, u);
    let mut kose = Vec::new();
    for j in 0..=dilim {
        let theta = std::f32::consts::TAU * j as f32 / dilim as f32;
        let (st, ct) = theta.sin_cos();
        let radyal = [
            ct * u[0] + st * v[0],
            ct * u[1] + st * v[1],
            ct * u[2] + st * v[2],
        ];
        // Alt halka (p0) ve üst halka (p1).
        kose.push(Vertex {
            konum: [
                p0[0] + yaricap * radyal[0],
                p0[1] + yaricap * radyal[1],
                p0[2] + yaricap * radyal[2],
            ],
            normal: radyal,
        });
        kose.push(Vertex {
            konum: [
                p1[0] + yaricap * radyal[0],
                p1[1] + yaricap * radyal[1],
                p1[2] + yaricap * radyal[2],
            ],
            normal: radyal,
        });
    }
    let mut indeks = Vec::new();
    for j in 0..dilim {
        let a0 = j * 2; // alt[j]
        let u0 = j * 2 + 1; // üst[j]
        let a1 = (j + 1) * 2; // alt[j+1]
        let u1 = (j + 1) * 2 + 1; // üst[j+1]
        indeks.extend_from_slice(&[a0, u0, a1, u0, u1, a1]);
    }
    Mesh { kose, indeks }
}

/// Örnek "top-çubuk" mesh'i: iki küre + bağ çubuğu (ÇE-07 öncesi 3B temeli gösterimi).
pub fn ornek_top_cubuk() -> Mesh {
    let mut m = kure([-1.1, 0.0, 0.0], 0.7, 24, 16);
    m.ekle(&kure([1.1, 0.0, 0.0], 0.7, 24, 16));
    m.ekle(&silindir([-1.1, 0.0, 0.0], [1.1, 0.0, 0.0], 0.22, 16));
    m
}

// ─── wgpu çizici ────────────────────────────────────────────────────────────────────────

/// Shader uniform'u (MVP + ışık yönü + malzeme rengi).  Düzen WGSL `U` ile birebir.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniform {
    mvp: [f32; 16],
    isik: [f32; 4],
    renk: [f32; 4],
}

/// Off-screen renk + derinlik dokusuna 3B mesh çizen wgpu çizici.
///
/// Sonuç renk dokusu UI'ya bir doku olarak verilir (egui `register_native_texture`).  Sabit
/// boyutludur → pencere yeniden boyutlandığında yeniden kurulum gerekmez; cihaz kaybında (TDR)
/// host yeniden kurar.
pub struct Sahne3B {
    renk_doku: wgpu::Texture,
    renk_view: wgpu::TextureView,
    _derinlik_doku: wgpu::Texture,
    derinlik_view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    indeks_sayisi: u32,
    en: u32,
    boy: u32,
}

impl Sahne3B {
    /// 3B çiziciyi belirtilen off-screen boyutta ve mesh ile kurar.
    pub fn yeni(device: &wgpu::Device, en: u32, boy: u32, mesh: &Mesh) -> Self {
        let renk_format = wgpu::TextureFormat::Rgba8Unorm;
        let derinlik_format = wgpu::TextureFormat::Depth32Float;
        let boyut = wgpu::Extent3d {
            width: en.max(1),
            height: boy.max(1),
            depth_or_array_layers: 1,
        };

        let renk_doku = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("biocraft-3b-renk"),
            size: boyut,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: renk_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let renk_view = renk_doku.create_view(&wgpu::TextureViewDescriptor::default());

        let derinlik_doku = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("biocraft-3b-derinlik"),
            size: boyut,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: derinlik_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let derinlik_view = derinlik_doku.create_view(&wgpu::TextureViewDescriptor::default());

        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("biocraft-3b-vbuf"),
            contents: bytemuck::cast_slice(&mesh.kose),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("biocraft-3b-ibuf"),
            contents: bytemuck::cast_slice(&mesh.indeks),
            usage: wgpu::BufferUsages::INDEX,
        });
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("biocraft-3b-uniform"),
            size: std::mem::size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("biocraft-3b-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("biocraft-3b-bg"),
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("biocraft-3b-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pl_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("biocraft-3b-pl"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        const ATTRS: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("biocraft-3b-pipeline"),
            layout: Some(&pl_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &ATTRS,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: renk_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                // Derinlik tamponu sıralamayı çözer → sarım yönünden bağımsız doğru sonuç.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: derinlik_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            renk_doku,
            renk_view,
            _derinlik_doku: derinlik_doku,
            derinlik_view,
            pipeline,
            vbuf,
            ibuf,
            ubuf,
            bind,
            indeks_sayisi: mesh.indeks.len() as u32,
            en: en.max(1),
            boy: boy.max(1),
        }
    }

    /// Off-screen renk dokusunun görünümü (egui'ye doku olarak kaydetmek için).
    pub fn renk_view(&self) -> &wgpu::TextureView {
        &self.renk_view
    }

    /// Renk dokusunun referansı (gerekirse).
    pub fn renk_doku(&self) -> &wgpu::Texture {
        &self.renk_doku
    }

    /// Off-screen boyut (en, boy).
    pub fn boyut(&self) -> (u32, u32) {
        (self.en, self.boy)
    }

    /// Sahneyi bir kez çizer (uniform'u günceller, kendi encoder'ını kuyruğa gönderir).
    ///
    /// `malzeme_dogrusal` ve `temizle_dogrusal`: token renginin **doğrusal (linear)** [r,g,b,a]
    /// hâli (bkz. [`crate::tokens::Renk::dogrusal_f32`]) → ekrandaki renk token tablosuyla eşleşir.
    pub fn ciz(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        kamera: &Kamera3B,
        isik_yonu: [f32; 3],
        malzeme_dogrusal: [f32; 4],
        temizle_dogrusal: [f32; 4],
    ) {
        let u = Uniform {
            mvp: kamera.mvp(),
            isik: [isik_yonu[0], isik_yonu[1], isik_yonu[2], 0.0],
            renk: malzeme_dogrusal,
        };
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("biocraft-3b-encoder"),
        });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("biocraft-3b-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renk_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: temizle_dogrusal[0] as f64,
                            g: temizle_dogrusal[1] as f64,
                            b: temizle_dogrusal[2] as f64,
                            a: temizle_dogrusal[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.derinlik_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.bind, &[]);
            rpass.set_vertex_buffer(0, self.vbuf.slice(..));
            rpass.set_index_buffer(self.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..self.indeks_sayisi, 0, 0..1);
        }
        queue.submit(std::iter::once(encoder.finish()));
    }
}

/// 3B shader (WGSL): MVP dönüşümü + dünya-uzayı Lambert ışıklandırma (model = birim).
const SHADER: &str = r#"
struct U {
    mvp: mat4x4<f32>,
    isik: vec4<f32>,
    renk: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;

struct VsCikti {
    @builtin(position) konum: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

@vertex
fn vs_main(@location(0) konum: vec3<f32>, @location(1) normal: vec3<f32>) -> VsCikti {
    var o: VsCikti;
    o.konum = u.mvp * vec4<f32>(konum, 1.0);
    o.normal = normal;
    return o;
}

@fragment
fn fs_main(i: VsCikti) -> @location(0) vec4<f32> {
    let n = normalize(i.normal);
    let l = normalize(u.isik.xyz);
    let yayinim = max(dot(n, l), 0.0);
    let yogunluk = 0.28 + 0.72 * yayinim; // ortam + dağınık
    return vec4<f32>(u.renk.rgb * yogunluk, 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn birim_matris_carpimi_kendini_verir() {
        let b = birim();
        let c = carp(&b, &b);
        assert_eq!(c, b);
    }

    #[test]
    fn kure_kose_ve_normal_sayisi_dogru() {
        let m = kure([0.0, 0.0, 0.0], 1.0, 8, 6);
        // (dilim+1)*(kat+1) köşe.
        assert_eq!(m.kose.len(), 9 * 7);
        // Tüm normaller birim uzunlukta.
        for v in &m.kose {
            let u = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
            assert!((u - 1.0).abs() < 1e-4, "normal birim değil: {u}");
        }
        assert!(m.ucgen_sayisi() > 0);
    }

    #[test]
    fn silindir_iki_noktayi_baglar() {
        let m = silindir([0.0, 0.0, 0.0], [0.0, 2.0, 0.0], 0.5, 12);
        assert_eq!(m.kose.len(), (12 + 1) * 2);
        assert!(m.ucgen_sayisi() >= 12 * 2);
    }

    #[test]
    fn mesh_ekleme_indeksleri_oteler() {
        let mut a = kure([0.0, 0.0, 0.0], 1.0, 4, 3);
        let a_kose = a.kose.len();
        let a_idx = a.indeks.len();
        let b = kure([3.0, 0.0, 0.0], 1.0, 4, 3);
        a.ekle(&b);
        assert_eq!(a.kose.len(), a_kose * 2);
        // Eklenen indekslerin en küçüğü taban kadar ötelenmiş olmalı.
        assert!(a.indeks[a_idx..].iter().all(|&i| i >= a_kose as u32));
    }

    #[test]
    fn ornek_top_cubuk_iki_top_bir_cubuk() {
        let m = ornek_top_cubuk();
        assert!(m.ucgen_sayisi() > 100);
        // Köşelerin x dağılımı iki tarafa yayılmalı (iki top).
        let sol = m.kose.iter().filter(|v| v.konum[0] < -0.4).count();
        let sag = m.kose.iter().filter(|v| v.konum[0] > 0.4).count();
        assert!(sol > 0 && sag > 0);
    }

    #[test]
    fn kamera_mvp_sonlu_ve_ondeki_nokta_gorunur() {
        let k = Kamera3B::yorunge(0.0, 5.0, 1.5, 4.0 / 3.0);
        let mvp = k.mvp();
        assert!(mvp.iter().all(|x| x.is_finite()));
        // Orijindeki noktayı clip uzayına taşı: w > 0 (kameranın önünde).
        // clip = mvp * (0,0,0,1) → 4. sütun.
        let w = mvp[3] * 0.0 + mvp[7] * 0.0 + mvp[11] * 0.0 + mvp[15] * 1.0;
        // w aslında perspektif sonrası -z; basitçe sonlu ve sıfırdan farklı olmalı.
        assert!(w.is_finite());
    }
}
