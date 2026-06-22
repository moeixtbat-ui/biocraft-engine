//! Status Bar (22 px alt) — bağlantı, FPS/bellek/**donanım göstergesi**, token sayacı, aktif iş (İP-03).
//!
//! 0.9 tablosu: "Durum, FPS/bellek/donanım (opsiyonel), token sayacı, bağlantı durumu".  Canlı
//! veriler dışarıdan ([`DurumBilgisi`]) verilir; bu modül yalnızca çizer.  Donanım göstergesi
//! İP-08 watchdog'undan ([`KoruyucuDurum`]) gelir; token sayacı AI yüzeyi (İP-14) bağlanınca
//! dolar — şimdilik "—" yer tutucudur (sahte sayı gösterilmez).
// MK-52: tüm renkler aktif token temasından; sabit renk yok.

use biocraft_mem::{KoruyucuDurum, OtoAyar, TermalAksiyon};
use biocraft_render::Backend;

use crate::components::StatusBadge;
use crate::i18n::Dil;
use crate::shell::layout::DURUM_YUKSEKLIK;
use crate::tokens::Tokenlar;

/// Status Bar'ın göstereceği canlı bilgiler (çizim katmanına veri taşır).
pub struct DurumBilgisi<'a> {
    /// Anlık FPS (kare bütçesinden — MK-03).
    pub fps: f32,
    /// Aktif render backend'i (GPU/CPU).
    pub backend: Backend,
    /// Geçici bildirim metni (örn. "GPU yeniden başlatıldı") — varsa sağda gösterilir.
    pub bildirim: Option<&'a str>,
    /// İP-08 watchdog'unun anlık donanım/termal durumu.
    pub donanim: &'a KoruyucuDurum,
    /// İP-08 başlangıç otomatik ayarı (düşük donanım sadeleşme uyarısı).
    pub oto: &'a OtoAyar,
    /// Ağ bağlantısı çevrimiçi mi? (gerçek ağ İP-15; şimdilik çağıran belirler.)
    pub cevrimici: bool,
    /// AI token sayacı (İP-14 ile dolar; `None` → "—").
    pub token_sayaci: Option<u64>,
    /// Aktif arka plan işinin kısa özeti (yoksa "Hazır").
    pub aktif_islem: Option<&'a str>,
}

/// Alt durum çubuğunu çizer (22 px).  Renkler token'dan (MK-52), metinler i18n'den (MK-53).
pub fn durum_cubugu(ctx: &egui::Context, bilgi: &DurumBilgisi, dil: Dil, tok: &Tokenlar) {
    let tr = matches!(dil, Dil::Tr);
    egui::TopBottomPanel::bottom("biocraft_durum")
        .exact_height(DURUM_YUKSEKLIK)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(format!("FPS: {:.0}", bilgi.fps));
                ui.separator();
                ui.label(format!("Backend: {}", bilgi.backend.etiket()));
                if bilgi.backend.yazilim_mi() {
                    ui.separator();
                    ui.colored_label(
                        tok.renk.uyari,
                        if tr {
                            "⚠ Yazılım (CPU) modu"
                        } else {
                            "⚠ Software (CPU) mode"
                        },
                    );
                }

                // İP-08: donanım göstergesi (CPU/GPU/RAM/sıcaklık).
                ui.separator();
                if bilgi.donanim.koruma_etkin {
                    ui.label(format!("🌡 {}", sicaklik_ozeti(bilgi.donanim)));
                } else {
                    ui.colored_label(
                        tok.renk.uyari,
                        if tr {
                            "🌡 Sensör yok"
                        } else {
                            "🌡 No sensor"
                        },
                    );
                }

                // Düşük donanım: sadeleşme + uyarı (MK-26).
                if bilgi.oto.sadelesme {
                    ui.separator();
                    ui.colored_label(
                        tok.renk.uyari,
                        format!(
                            "⚙ {} · {} FPS",
                            if tr { "Sadeleştirildi" } else { "Simplified" },
                            bilgi.oto.hedef_fps
                        ),
                    );
                }

                // Sağ taraf (sondan başa): bildirim → termal rozet → aktif iş → token → bağlantı.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(b) = bilgi.bildirim {
                        ui.colored_label(tok.renk.basari, format!("✔ {b}"));
                        ui.separator();
                    }

                    // Termal acil/soğutma rozeti (varsa).
                    if bilgi.donanim.acil_durum {
                        ui.colored_label(
                            tok.renk.hata,
                            if tr {
                                "⛔ ACİL DURDU"
                            } else {
                                "⛔ EMERGENCY STOP"
                            },
                        );
                        ui.separator();
                    } else if bilgi.donanim.sogutuluyor {
                        let _ = StatusBadge::Sogutuluyor.show(ui, dil, tok);
                        ui.separator();
                    } else if let TermalAksiyon::YukAzalt(p) = bilgi.donanim.aksiyon {
                        ui.colored_label(tok.renk.uyari, format!("⏬ %{p}"));
                        ui.separator();
                    }

                    // Bağlantı durumu rozeti (çevrimiçi/çevrimdışı).
                    let rozet = if bilgi.cevrimici {
                        StatusBadge::Cevrimici
                    } else {
                        StatusBadge::Cevrimdisi
                    };
                    let _ = rozet.show(ui, dil, tok);
                    ui.separator();

                    // AI token sayacı — İP-14 yüzeyi etkin + gösterge açıkken oturum jetonuyla dolar;
                    // aksi hâlde "—".
                    let token = match bilgi.token_sayaci {
                        Some(n) => format!("⊙ {n}"),
                        None => "⊙ —".to_string(),
                    };
                    let token_ipucu = match bilgi.token_sayaci {
                        Some(_) if tr => "AI oturum token sayacı",
                        Some(_) => "AI session token counter",
                        None if tr => "AI token sayacı (kapalı/yapılandırılmadı)",
                        None => "AI token counter (off/not configured)",
                    };
                    ui.label(egui::RichText::new(token).color(tok.renk.metin_soluk))
                        .on_hover_text(token_ipucu);
                    ui.separator();

                    // Aktif arka plan işi özeti (yoksa "Hazır").
                    let islem = bilgi
                        .aktif_islem
                        .unwrap_or(if tr { "Hazır" } else { "Ready" });
                    ui.label(egui::RichText::new(islem).color(tok.renk.metin_soluk));
                });
            });
        });
}

/// Watchdog örneğinden kısa "GPU 82°C · CPU %45" özeti üretir (mevcut değerler).
fn sicaklik_ozeti(donanim: &KoruyucuDurum) -> String {
    let o = &donanim.son_ornek;
    let mut parcalar: Vec<String> = Vec::new();
    if let Some(t) = o.gpu_c {
        parcalar.push(format!("GPU {t:.0}°C"));
    }
    if let Some(t) = o.cpu_c {
        parcalar.push(format!("CPU {t:.0}°C"));
    }
    if let Some(t) = o.nvme_c {
        parcalar.push(format!("NVMe {t:.0}°C"));
    }
    if let Some(p) = o.cpu_yuzde {
        parcalar.push(format!("CPU %{p:.0}"));
    }
    if let Some(r) = o.ram_orani {
        parcalar.push(format!("RAM %{:.0}", r * 100.0));
    }
    if parcalar.is_empty() {
        "ölçülüyor…".to_string()
    } else {
        parcalar.join(" · ")
    }
}
