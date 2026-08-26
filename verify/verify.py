# -*- coding: utf-8 -*-
"""
Независимая верификация расчётов SpeakerLab.

Метод: физика (электро-механо-акустическая схема в импедансном аналоге)
реализована здесь с нуля на numpy — отдельным кодом от Rust-ядра, — и
сверяется точечно с дампом кривых SpeakerLab (curves.json). Дополнительно —
сверка с замкнутыми формулами Тиля–Смолла из учебников и качественные
признаки правильной физики.

Запуск:
    cargo run -p speakerlab-acoustics --example dump_curves > verify/curves.json
    python verify/verify.py
"""
import json
import math
import sys

import numpy as np

RHO = 1.204        # кг/м³
C = 343.0          # м/с
P_REF = 2e-5       # Па

results = []


def check(name, ok, detail=""):
    results.append((name, ok, detail))
    print(f"  {'✅' if ok else '❌'} {name} — {detail}")


def par(a, b):
    return a * b / (a + b)


def load():
    with open(__file__.replace("verify.py", "curves.json"), encoding="utf-8") as f:
        return json.load(f)


def driver_params(d):
    """Производные T/S-параметры — по определению из учебников."""
    re, fs, qms, qes = d["re"], d["fs"], d["qms"], d["qes"]
    vas_m3 = d["vas_l"] / 1e3
    sd_m2 = d["sd_cm2"] / 1e4
    cas = vas_m3 / (RHO * C**2)          # акустическая ёмкость
    cms = cas / sd_m2**2                 # м/Н
    mms = 1.0 / ((2 * math.pi * fs) ** 2 * cms)
    rms = 2 * math.pi * fs * mms / qms
    bl = math.sqrt(2 * math.pi * fs * mms * re / qes)
    return dict(re=re, le=d["le_mH"] / 1e3, sd=sd_m2, cms=cms, mms=mms, rms=rms, bl=bl,
                qts=qes * qms / (qes + qms), vas_m3=vas_m3, fs=fs)


def solve(p, za, w, voltage):
    """Решение схемы (механический домен): v = Bl·e / ((Re+jωLe)·Zm + Bl²)."""
    ze = p["re"] + 1j * w * p["le"]
    zm = p["rms"] + 1j * w * p["mms"] + 1 / (1j * w * p["cms"]) + za * p["sd"] ** 2
    v = p["bl"] * voltage / (ze * zm + p["bl"] ** 2)
    i = v * zm / p["bl"]
    zin = voltage / i
    return v * p["sd"], zin, v  # U_d, Z_in, скорость


def sealed_za(p, vb_l, w):
    cab = (vb_l / 1e3) / (RHO * C**2)
    return 1 / (1j * w * cab)


def vented_za(p, vb_l, fb, ql, w, q_port=10.0):
    cab = (vb_l / 1e3) / (RHO * C**2)
    wb = 2 * math.pi * fb
    m_ap = 1 / (wb**2 * cab)
    r_al = ql / (wb * cab)
    r_ap = wb * m_ap / q_port
    z_c = par(r_al + 0j, 1 / (1j * w * cab))
    z_p = r_ap + 1j * w * m_ap
    return par(z_c, z_p), z_p


def spl_from_u(u_rad, w):
    p_far = w * RHO * np.abs(u_rad) / (2 * math.pi)
    return 20 * np.log10(np.maximum(p_far, 1e-30) / P_REF)


def main():
    data = load()
    p = driver_params(data["driver"])
    voltage = data["voltage"]

    # независимая копия динамика без Le (как в дампе alignment/spl для сравнения отклика)
    p0 = dict(p)
    p0["le"] = 0.0

    print("== 1. ЗЯ: независимая схема vs SpeakerLab ==")
    s = data["sealed"]
    f = np.array(s["freq"])
    w = 2 * np.pi * f
    za_s = sealed_za(p0, s["vb_l"], w)
    u_d = np.array([solve(p0, z, wi, voltage)[0] for z, wi in zip(za_s, w)])
    spl_py = spl_from_u(-u_d, w)
    d_spl = np.abs(spl_py - np.array(s["spl"]))
    check("SPL ЗЯ совпадает с независимой схемой", d_spl.max() < 0.01,
          f"max |Δ| = {d_spl.max():.2e} дБ (512 точек)")

    print("== 2. ЗЯ: замкнутая формула передаточной функции vs кривая ==")
    fc, qtc = s["fc"], s["qtc"]
    num = f**4
    den = (fc**2 - f**2) ** 2 + (f * fc / qtc) ** 2
    h = 10 * np.log10(num / den)
    band = (f >= 100) & (f <= 800)
    h_n = h - h[band].mean()
    rust_n = np.array(s["alignment_spl"]) - np.array(s["alignment_spl"])[band].mean()
    zone = (f >= 20) & (f <= 500)
    d_an = np.abs(h_n - rust_n)[zone]
    check("Формула H(s)=s²/(s²+ωc/Q·s+ωc²) совпадает с кривой", d_an.max() < 0.15,
          f"max |Δ| = {d_an.max():.3f} дБ в 20–500 Гц")

    print("== 3. ЗЯ: fc, Qtc, F3 по формулам ==")
    alpha = p["vas_m3"] / (s["vb_l"] / 1e3)
    check("fc = fs·√(1+α)", abs(p["fs"] * math.sqrt(1 + alpha) - fc) / fc < 1e-9,
          f"{p['fs'] * math.sqrt(1 + alpha):.3f} vs {fc:.3f} Гц")
    check("Qtc = Qts·√(1+α)", abs(p["qts"] * math.sqrt(1 + alpha) - qtc) / qtc < 1e-9,
          f"{p['qts'] * math.sqrt(1 + alpha):.4f} vs {qtc:.4f}")
    a = 1 - 1 / (2 * qtc**2)
    f3_theory = fc * math.sqrt(math.sqrt(a * a + 1) - a)
    check("F3 = fc·sqrt(sqrt(a²+1)−a), a=1−1/(2Q²)", abs(f3_theory - s["f3"]) / s["f3"] < 0.02,
          f"{f3_theory:.2f} vs {s['f3']:.2f} Гц")

    print("== 4. ФИ: независимая схема vs SpeakerLab (SPL, |Z|, экскурсия) ==")
    v = data["vented"]
    fv = np.array(v["freq"])
    wv = 2 * np.pi * fv
    za_v, zp = vented_za(p0, v["vb_l"], v["fb"], v["ql"], wv)
    u_rad_all, exc_all = [], []
    for k, wi in enumerate(wv):
        u_d, _, vel = solve(p0, za_v[k], wi, voltage)
        u_p = (u_d * za_v[k]) / zp[k]
        u_rad_all.append(u_p - u_d)
        exc_all.append(abs(vel) / wi * 1e3)
    spl_vp = spl_from_u(np.array(u_rad_all), wv)
    d_v = np.abs(spl_vp - np.array(v["spl"]))
    check("SPL ФИ совпадает с независимой схемой", d_v.max() < 0.01,
          f"max |Δ| = {d_v.max():.2e} дБ")

    zv = np.array(data["z_vented"])
    fz, zmag, exc = zv[:, 0], zv[:, 1], zv[:, 2]
    za_z, zp_z = vented_za(p, v["vb_l"], v["fb"], v["ql"], 2 * np.pi * fz)
    z_py = np.array([solve(p, z, wi, voltage)[1] for z, wi in zip(za_z, 2 * np.pi * fz)])
    d_z = np.abs(np.abs(z_py) - zmag)
    check("|Z| ФИ совпадает (с реальной Le)", d_z.max() < 0.01,
          f"max |Δ| = {d_z.max():.2e} Ом")

    exc_py = np.array(exc_all)
    d_e = np.abs(exc_py - np.array(v["excursion_mm"]))
    check("Экскурсия совпадает", d_e.max() < 1e-3, f"max |Δ| = {d_e.max():.2e} мм")

    print("== 5. ФИ: качественные признаки классической физики ==")
    fb = v["fb"]
    win = (fz >= 0.7 * fb) & (fz <= 1.3 * fb)
    f_min = fz[win][np.argmin(zmag[win])]
    check("Минимум |Z| на Fb", abs(f_min - fb) / fb < 0.02, f"{f_min:.1f} vs {fb} Гц")
    wexc = (fv >= 0.75 * fb) & (fv <= 1.25 * fb)
    f_dip = fv[wexc][np.argmin(np.array(v["excursion_mm"])[wexc])]
    check("Провал экскурсии на Fb", abs(f_dip - fb) / fb < 0.03, f"{f_dip:.1f} vs {fb} Гц")
    spl_v_arr = np.array(v["spl"])
    f_lo, f_hi = 0.4 * fb, 0.8 * fb
    i_lo = int(np.argmin(np.abs(fv - f_lo)))
    i_hi = int(np.argmin(np.abs(fv - f_hi)))
    slope = (spl_v_arr[i_hi] - spl_v_arr[i_lo]) / math.log2(fv[i_hi] / fv[i_lo])
    check("Нижний спад ~24 дБ/окт", 14 < slope < 26, f"{slope:.1f} дБ/окт (14–28 Гц)")
    interior = zmag[1:-1]
    mask = (interior > zmag[:-2]) & (interior > zmag[2:]) & (fz[1:-1] > 15) & (fz[1:-1] < 120)
    peaks = fz[1:-1][mask]
    check("Двугорбый импеданс (пики по обе стороны Fb)",
          peaks.size >= 2 and peaks.min() < fb < peaks.max(),
          f"пики: {[round(x,1) for x in peaks.tolist()]}")

    print("== 6. Порт: классическая формула vs SpeakerLab ==")
    dv_cm, vb_l, fb_hz = 10.0, 50.0, 35.0
    classic = 23562.5 * dv_cm**2 / (fb_hz**2 * vb_l) - 0.732 * dv_cm  # c≈343.8
    si = (math.pi * (dv_cm / 2) ** 2) * 1e-4 * C**2 / ((2 * math.pi * fb_hz) ** 2 * (vb_l / 1e3)) * 100 - 0.732 * dv_cm
    check("L = c²·A/(ω²·V) − 0.732·D (СИ)", abs(si - data["port_len_cm"]) < 0.05,
          f"{si:.2f} vs {data['port_len_cm']:.2f} см; справочник(c=344): {classic:.2f} см")

    print("== 7. Абсолютный уровень: эталонная эффективность η₀ ==")
    d = data["driver"]
    eta0 = 9.64e-10 * d["fs"] ** 3 * d["vas_l"] / d["qes"]       # Small, Vas в литрах
    spl_1w = 112.02 + 10 * math.log10(eta0)
    p_e = voltage**2 / d["re"]
    expected = spl_1w + 10 * math.log10(p_e)                     # 2.83 В → 2.5 Вт
    got = data["sim_passband_spl"]
    check("SPL(полоса) = 112+10lg(η₀)+10lg(V²/Re)", abs(expected - got) < 1.5,
          f"теория {expected:.2f} vs симуляция {got:.2f} дБ")

    print("== 8. Метрики в рабочих окнах и предельные напряжения ==")
    check("|Z|max в окне 15–500 Гц (резонанс, не рост Le)",
          15 <= v["z_max_freq"] <= 500 and v["z_max_freq"] < 200,
          f"{v['z_max_freq']:.1f} Гц")
    check("Максимум хода в окне 15–300 Гц",
          15 <= v["exc_max_freq"] <= 300,
          f"{v['exc_max_freq']:.1f} Гц")
    efb = v["exc_at_fb"]
    check("Ход на Fb заметно меньше максимума (порт разгружает)",
          efb is not None and efb < 0.8 * v["exc_max"],
          f"{efb:.2f} vs {v['exc_max']:.2f} мм")

    # Независимый расчёт v_xmax из собственной кривой хода (та же линейность,
    # другой код) + сквозная проверка: при v_xmax максимум хода == Xmax
    exc_py_arr = np.array(exc_all)
    band_mask = (fv >= 15) & (fv <= 500)
    idx = np.where(band_mask & (exc_py_arr > 1e-9))[0]
    ratios = data["xmax_mm"] / exc_py_arr[idx]
    j = idx[np.argmin(ratios)]
    v_xmax_py = voltage * ratios.min()
    v_xmax_dump = data["limits"]["v_xmax"][0]
    check("v_xmax совпадает с независимым расчётом",
          abs(v_xmax_py - v_xmax_dump) / v_xmax_dump < 0.01,
          f"{v_xmax_py:.3f} vs {v_xmax_dump:.3f} В")

    # Сквозная: симуляция при v_xmax → максимум хода == Xmax ±2%
    za_e, zp_e = vented_za(p0, v["vb_l"], v["fb"], v["ql"], wv)
    exc_at_limit = []
    for k in np.where(band_mask)[0]:
        _, _, vel = solve(p0, za_e[k], wv[k], v_xmax_py)
        exc_at_limit.append(abs(vel) / wv[k] * 1e3)
    exc_peak = max(exc_at_limit)
    check("При v_xmax максимум хода = Xmax (±2%)",
          abs(exc_peak / data["xmax_mm"] - 1.0) < 0.02,
          f"{exc_peak:.3f} мм vs Xmax {data['xmax_mm']} мм")

    print()
    failed = sum(1 for _, ok, _ in results if not ok)
    total = len(results)
    print(f"ИТОГ: {total - failed}/{total} проверок пройдено")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
