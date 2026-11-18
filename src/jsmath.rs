//! `Math.*` as V8 computes it, bit for bit. V8 implements these functions with fdlibm
//! (`src/base/ieee754.cc`); Rust's `f64` methods call the platform's libm, whose results differ
//! in the last bit for a few percent of arguments. The fdlibm algorithms are ported here; the
//! `libm` crate (musl, itself fdlibm-derived) supplies the ones that already agree with V8.
//! `sin` and `cos` (V8 uses a copy of glibc's) may still differ in the last bit.

// fdlibm constants preserve the original bit patterns; equal-operand operations deliberately
// produce IEEE NaNs/infinities. Rewriting them can change rounding and exception behavior.
#![allow(clippy::approx_constant, clippy::eq_op, clippy::excessive_precision)]

fn hi(x: f64) -> i32 {
    (x.to_bits() >> 32) as i32
}

fn lo(x: f64) -> u32 {
    x.to_bits() as u32
}

fn from_words(hi: i32, lo: u32) -> f64 {
    f64::from_bits(((hi as u32 as u64) << 32) | lo as u64)
}

fn with_hi(x: f64, h: i32) -> f64 {
    from_words(h, lo(x))
}

fn with_lo(x: f64, l: u32) -> f64 {
    from_words(hi(x), l)
}

const LN2_HI: f64 = 6.93147180369123816490e-01; /* 3fe62e42 fee00000 */
const LN2_LO: f64 = 1.90821492927058770002e-10; /* 3dea39ef 35793c76 */
const TWO54: f64 = 1.80143985094819840000e+16; /* 43500000 00000000 */

/// fdlibm `__ieee754_log`.
pub fn log(x: f64) -> f64 {
    const LG1: f64 = 6.666666666666735130e-01; /* 3FE55555 55555593 */
    const LG2: f64 = 3.999999999940941908e-01; /* 3FD99999 9997FA04 */
    const LG3: f64 = 2.857142874366239149e-01; /* 3FD24924 94229359 */
    const LG4: f64 = 2.222219843214978396e-01; /* 3FCC71C5 1D8E78AF */
    const LG5: f64 = 1.818357216161805012e-01; /* 3FC74664 96CB03DE */
    const LG6: f64 = 1.531383769920937332e-01; /* 3FC39A09 D078C69F */
    const LG7: f64 = 1.479819860511658591e-01; /* 3FC2F112 DF3E5244 */
    let mut x = x;
    let mut hx = hi(x);
    let lx = lo(x);
    let mut k: i32 = 0;
    if hx < 0x00100000 {
        if ((hx & 0x7fffffff) as u32 | lx) == 0 {
            return f64::NEG_INFINITY;
        }
        if hx < 0 {
            return f64::NAN;
        }
        k -= 54;
        x *= TWO54;
        hx = hi(x);
    }
    if hx >= 0x7ff00000 {
        return x + x;
    }
    k += (hx >> 20) - 1023;
    hx &= 0x000fffff;
    let i = (hx + 0x95f64) & 0x100000;
    x = with_hi(x, hx | (i ^ 0x3ff00000));
    k += i >> 20;
    let f = x - 1.0;
    if (0x000fffff & (2 + hx)) < 3 {
        if f == 0.0 {
            if k == 0 {
                return 0.0;
            }
            let dk = k as f64;
            return dk * LN2_HI + dk * LN2_LO;
        }
        let r = f * f * (0.5 - 0.33333333333333333 * f);
        if k == 0 {
            return f - r;
        }
        let dk = k as f64;
        return dk * LN2_HI - ((r - dk * LN2_LO) - f);
    }
    let s = f / (2.0 + f);
    let dk = k as f64;
    let z = s * s;
    let mut i = hx - 0x6147a;
    let w = z * z;
    let j = 0x6b851 - hx;
    let t1 = w * (LG2 + w * (LG4 + w * LG6));
    let t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    i |= j;
    let r = t2 + t1;
    if i > 0 {
        let hfsq = 0.5 * f * f;
        if k == 0 {
            f - (hfsq - s * (hfsq + r))
        } else {
            dk * LN2_HI - ((hfsq - (s * (hfsq + r) + dk * LN2_LO)) - f)
        }
    } else if k == 0 {
        f - s * (f - r)
    } else {
        dk * LN2_HI - ((s * (f - r) - dk * LN2_LO) - f)
    }
}

/// V8's `log10` (fdlibm `__ieee754_log10`).
pub fn log10(x: f64) -> f64 {
    const IVLN10: f64 = 4.34294481903251816668e-01;
    const LOG10_2HI: f64 = 3.01029995663611771306e-01; /* 0x3FD34413, 0x509F6000 */
    const LOG10_2LO: f64 = 3.69423907715893078616e-13; /* 0x3D59FEF3, 0x11F12B36 */
    let mut x = x;
    let mut hx = hi(x);
    let mut lx = lo(x);
    let mut k: i32 = 0;
    if hx < 0x00100000 {
        if ((hx & 0x7fffffff) as u32 | lx) == 0 {
            return f64::NEG_INFINITY;
        }
        if hx < 0 {
            return f64::NAN;
        }
        k -= 54;
        x *= TWO54;
        hx = hi(x);
        lx = lo(x);
    }
    if hx >= 0x7ff00000 {
        return x + x;
    }
    if hx == 0x3ff00000 && lx == 0 {
        return 0.0;
    }
    k += (hx >> 20) - 1023;
    let i = ((k as u32 & 0x80000000) >> 31) as i32;
    hx = (hx & 0x000fffff) | ((0x3ff - i) << 20);
    let y = (k + i) as f64;
    x = from_words(hx, lx);
    let z = y * LOG10_2LO + IVLN10 * log(x);
    z + y * LOG10_2HI
}

/// fdlibm `log1p`.
pub fn log1p(x: f64) -> f64 {
    const LP1: f64 = 6.666666666666735130e-01; /* 3FE55555 55555593 */
    const LP2: f64 = 3.999999999940941908e-01; /* 3FD99999 9997FA04 */
    const LP3: f64 = 2.857142874366239149e-01; /* 3FD24924 94229359 */
    const LP4: f64 = 2.222219843214978396e-01; /* 3FCC71C5 1D8E78AF */
    const LP5: f64 = 1.818357216161805012e-01; /* 3FC74664 96CB03DE */
    const LP6: f64 = 1.531383769920937332e-01; /* 3FC39A09 D078C69F */
    const LP7: f64 = 1.479819860511658591e-01; /* 3FC2F112 DF3E5244 */
    let hx = hi(x);
    let ax = hx & 0x7fffffff;
    let mut k: i32 = 1;
    let mut f = 0.0;
    let mut hu: i32 = 0;
    let mut c = 0.0;
    if hx < 0x3FDA827A {
        if ax >= 0x3ff00000 {
            return if x == -1.0 { f64::NEG_INFINITY } else { f64::NAN };
        }
        if ax < 0x3e200000 {
            if TWO54 + x > 0.0 && ax < 0x3c900000 {
                return x;
            }
            return x - x * x * 0.5;
        }
        if hx > 0 || hx <= 0xbfd2bec3u32 as i32 {
            k = 0;
            f = x;
            hu = 1;
        }
    }
    if hx >= 0x7ff00000 {
        return x + x;
    }
    if k != 0 {
        let mut u;
        if hx < 0x43400000 {
            u = 1.0 + x;
            hu = hi(u);
            k = (hu >> 20) - 1023;
            c = if k > 0 { 1.0 - (u - x) } else { x - (u - 1.0) };
            c /= u;
        } else {
            u = x;
            hu = hi(u);
            k = (hu >> 20) - 1023;
            c = 0.0;
        }
        hu &= 0x000fffff;
        if hu < 0x6a09e {
            u = with_hi(u, hu | 0x3ff00000);
        } else {
            k += 1;
            u = with_hi(u, hu | 0x3fe00000);
            hu = (0x00100000 - hu) >> 2;
        }
        f = u - 1.0;
    }
    let hfsq = 0.5 * f * f;
    let dk = k as f64;
    if hu == 0 {
        if f == 0.0 {
            if k == 0 {
                return 0.0;
            }
            c += dk * LN2_LO;
            return dk * LN2_HI + c;
        }
        let r = hfsq * (1.0 - 0.66666666666666666 * f);
        if k == 0 {
            return f - r;
        }
        return dk * LN2_HI - ((r - (dk * LN2_LO + c)) - f);
    }
    let s = f / (2.0 + f);
    let z = s * s;
    let r = z * (LP1 + z * (LP2 + z * (LP3 + z * (LP4 + z * (LP5 + z * (LP6 + z * LP7))))));
    if k == 0 {
        f - (hfsq - s * (hfsq + r))
    } else {
        dk * LN2_HI - ((hfsq - (s * (hfsq + r) + (dk * LN2_LO + c))) - f)
    }
}

/// V8's `cbrt` (FreeBSD `s_cbrt.c`).
pub fn cbrt(x: f64) -> f64 {
    const B1: u32 = 715094163;
    const B2: u32 = 696219795;
    const P0: f64 = 1.87595182427177009643;
    const P1: f64 = -1.88497979543377169875;
    const P2: f64 = 1.621429720105354466140;
    const P3: f64 = -0.758397934778766047437;
    const P4: f64 = 0.145996192886612446982;
    let mut hx = hi(x) as u32;
    let low = lo(x);
    let sign = hx & 0x80000000;
    hx ^= sign;
    if hx >= 0x7ff00000 {
        return x + x;
    }
    let mut t;
    if hx < 0x00100000 {
        if (hx | low) == 0 {
            return x;
        }
        t = from_words(0x43500000, 0);
        t *= x;
        let high = hi(t) as u32;
        t = from_words((sign | ((high & 0x7fffffff) / 3 + B2)) as i32, 0);
    } else {
        t = from_words((sign | (hx / 3 + B1)) as i32, 0);
    }
    let r = (t * t) * (t / x);
    t *= (P0 + r * (P1 + r * P2)) + ((r * r) * r) * (P3 + r * P4);
    t = f64::from_bits((t.to_bits().wrapping_add(0x80000000)) & 0xffffffffc0000000);
    let s = t * t;
    let mut r = x / s;
    let w = t + t;
    r = (r - t) / (w + r);
    t + t * r
}

/// fdlibm `__ieee754_pow` (the caller handles JavaScript's NaN cases first).
// fdlibm's branch structure is kept as written, identical arms included.
#[allow(clippy::if_same_then_else)]
pub fn pow(x: f64, y: f64) -> f64 {
    const BP: [f64; 2] = [1.0, 1.5];
    const DP_H: [f64; 2] = [0.0, 5.84962487220764160156e-01]; /* 0x3FE2B803, 0x40000000 */
    const DP_L: [f64; 2] = [0.0, 1.35003920212974897128e-08]; /* 0x3E4CFDEB, 0x43CFD006 */
    const TWO53: f64 = 9007199254740992.0;
    const HUGE: f64 = 1.0e300;
    const TINY: f64 = 1.0e-300;
    const L1: f64 = 5.99999999999994648725e-01; /* 0x3FE33333, 0x33333303 */
    const L2: f64 = 4.28571428578550184252e-01; /* 0x3FDB6DB6, 0xDB6FABFF */
    const L3: f64 = 3.33333329818377432918e-01; /* 0x3FD55555, 0x518F264D */
    const L4: f64 = 2.72728123808534006489e-01; /* 0x3FD17460, 0xA91D4101 */
    const L5: f64 = 2.30660745775561754067e-01; /* 0x3FCD864A, 0x93C9DB65 */
    const L6: f64 = 2.06975017800338417784e-01; /* 0x3FCA7E28, 0x4A454EEF */
    const P1: f64 = 1.66666666666666019037e-01; /* 0x3FC55555, 0x5555553E */
    const P2: f64 = -2.77777777770155933842e-03; /* 0xBF66C16C, 0x16BEBD93 */
    const P3: f64 = 6.61375632143793436117e-05; /* 0x3F11566A, 0xAF25DE2C */
    const P4: f64 = -1.65339022054652515390e-06; /* 0xBEBBBD41, 0xC5D26BF1 */
    const P5: f64 = 4.13813679705723846039e-08; /* 0x3E663769, 0x72BEA4D0 */
    const LG2: f64 = 6.93147180559945286227e-01; /* 0x3FE62E42, 0xFEFA39EF */
    const LG2_H: f64 = 6.93147182464599609375e-01; /* 0x3FE62E43, 0x00000000 */
    const LG2_L: f64 = -1.90465429995776804525e-09; /* 0xBE205C61, 0x0CA86C39 */
    const OVT: f64 = 8.0085662595372944372e-17;
    const CP: f64 = 9.61796693925975554329e-01; /* 0x3FEEC709, 0xDC3A03FD */
    const CP_H: f64 = 9.61796700954437255859e-01; /* 0x3FEEC709, 0xE0000000 */
    const CP_L: f64 = -7.02846165095275826516e-09; /* 0xBE3E2FE0, 0x145B01F5 */
    const IVLN2: f64 = 1.44269504088896338700e+00; /* 0x3FF71547, 0x652B82FE */
    const IVLN2_H: f64 = 1.44269502162933349609e+00; /* 0x3FF71547, 0x60000000 */
    const IVLN2_L: f64 = 1.92596299112661746887e-08; /* 0x3E54AE0B, 0xF85DDF44 */

    let hx = hi(x);
    let lx = lo(x);
    let hy = hi(y);
    let ly = lo(y);
    let mut ix = hx & 0x7fffffff;
    let iy = hy & 0x7fffffff;

    if (iy as u32 | ly) == 0 {
        return 1.0;
    }
    if ix > 0x7ff00000 || (ix == 0x7ff00000 && lx != 0) || iy > 0x7ff00000 || (iy == 0x7ff00000 && ly != 0) {
        return x + y;
    }
    // yisint: 0 not an integer, 1 odd, 2 even
    let mut yisint = 0;
    if hx < 0 {
        if iy >= 0x43400000 {
            yisint = 2;
        } else if iy >= 0x3ff00000 {
            let k = (iy >> 20) - 0x3ff;
            if k > 20 {
                let j = ly >> (52 - k);
                if (j << (52 - k)) == ly {
                    yisint = 2 - (j & 1) as i32;
                }
            } else if ly == 0 {
                let j = iy >> (20 - k);
                if (j << (20 - k)) == iy {
                    yisint = 2 - (j & 1);
                }
            }
        }
    }
    if ly == 0 {
        if iy == 0x7ff00000 {
            if ((ix - 0x3ff00000) as u32 | lx) == 0 {
                return y - y;
            } else if ix >= 0x3ff00000 {
                return if hy >= 0 { y } else { 0.0 };
            } else {
                return if hy < 0 { -y } else { 0.0 };
            }
        }
        if iy == 0x3ff00000 {
            return if hy < 0 { 1.0 / x } else { x };
        }
        if hy == 0x40000000 {
            return x * x;
        }
        if hy == 0x3fe00000 && hx >= 0 {
            return x.sqrt();
        }
    }
    let mut ax = x.abs();
    if lx == 0 && (ix == 0x7ff00000 || ix == 0 || ix == 0x3ff00000) {
        let mut z = ax;
        if hy < 0 {
            z = 1.0 / z;
        }
        if hx < 0 {
            if ((ix - 0x3ff00000) | yisint) == 0 {
                z = (z - z) / (z - z);
            } else if yisint == 1 {
                z = -z;
            }
        }
        return z;
    }
    // fdlibm's `(hx >> 31) + 1`: 0 for a negative x, else 1.
    let n: i32 = if hx < 0 { 0 } else { 1 };
    if (n | yisint) == 0 {
        return (x - x) / (x - x);
    }
    let mut s = 1.0;
    if (n | (yisint - 1)) == 0 {
        s = -1.0;
    }
    let t1;
    let t2;
    if iy > 0x41e00000 {
        if iy > 0x43f00000 {
            if ix <= 0x3fefffff {
                return if hy < 0 { HUGE * HUGE } else { TINY * TINY };
            }
            if ix >= 0x3ff00000 {
                return if hy > 0 { HUGE * HUGE } else { TINY * TINY };
            }
        }
        if ix < 0x3fefffff {
            return if hy < 0 { s * HUGE * HUGE } else { s * TINY * TINY };
        }
        if ix > 0x3ff00000 {
            return if hy > 0 { s * HUGE * HUGE } else { s * TINY * TINY };
        }
        let t = ax - 1.0;
        let w = (t * t) * (0.5 - t * (0.3333333333333333333333 - t * 0.25));
        let u = IVLN2_H * t;
        let v = t * IVLN2_L - w * IVLN2;
        t1 = with_lo(u + v, 0);
        t2 = v - (t1 - u);
    } else {
        let mut n2: i32 = 0;
        if ix < 0x00100000 {
            ax *= TWO53;
            n2 -= 53;
            ix = hi(ax);
        }
        n2 += (ix >> 20) - 0x3ff;
        let j = ix & 0x000fffff;
        ix = j | 0x3ff00000;
        let k: usize;
        if j <= 0x3988E {
            k = 0;
        } else if j < 0xBB67A {
            k = 1;
        } else {
            k = 0;
            n2 += 1;
            ix -= 0x00100000;
        }
        ax = with_hi(ax, ix);
        let u = ax - BP[k];
        let v = 1.0 / (ax + BP[k]);
        let ss = u * v;
        let s_h = with_lo(ss, 0);
        let t_h = from_words(((ix >> 1) | 0x20000000) + 0x00080000 + ((k as i32) << 18), 0);
        let t_l = ax - (t_h - BP[k]);
        let s_l = v * ((u - s_h * t_h) - s_h * t_l);
        let mut s2 = ss * ss;
        let mut r = s2 * s2 * (L1 + s2 * (L2 + s2 * (L3 + s2 * (L4 + s2 * (L5 + s2 * L6)))));
        r += s_l * (s_h + ss);
        s2 = s_h * s_h;
        let t_h = with_lo(3.0 + s2 + r, 0);
        let t_l = r - ((t_h - 3.0) - s2);
        let u = s_h * t_h;
        let v = s_l * t_h + t_l * ss;
        let p_h = with_lo(u + v, 0);
        let p_l = v - (p_h - u);
        let z_h = CP_H * p_h;
        let z_l = CP_L * p_h + p_l * CP + DP_L[k];
        let t = n2 as f64;
        t1 = with_lo(((z_h + z_l) + DP_H[k]) + t, 0);
        t2 = z_l - (((t1 - t) - DP_H[k]) - z_h);
    }
    let y1 = with_lo(y, 0);
    let p_l = (y - y1) * t1 + y * t2;
    let mut p_h = y1 * t1;
    let mut z = p_l + p_h;
    let j = hi(z);
    let i = lo(z);
    if j >= 0x40900000 {
        if ((j - 0x40900000) as u32 | i) != 0 {
            return s * HUGE * HUGE;
        } else if p_l + OVT > z - p_h {
            return s * HUGE * HUGE;
        }
    } else if (j & 0x7fffffff) >= 0x4090cc00 {
        if ((j as u32).wrapping_sub(0xc090cc00) | i) != 0 {
            return s * TINY * TINY;
        } else if p_l <= z - p_h {
            return s * TINY * TINY;
        }
    }
    let i = j & 0x7fffffff;
    let mut k = (i >> 20) - 0x3ff;
    let mut n: i32 = 0;
    if i > 0x3fe00000 {
        n = j + (0x00100000 >> (k + 1));
        k = ((n & 0x7fffffff) >> 20) - 0x3ff;
        let t = from_words(n & !(0x000fffff >> k), 0);
        n = ((n & 0x000fffff) | 0x00100000) >> (20 - k);
        if j < 0 {
            n = -n;
        }
        p_h -= t;
    }
    let t = with_lo(p_l + p_h, 0);
    let u = t * LG2_H;
    let v = (p_l - (t - p_h)) * LG2 + t * LG2_L;
    z = u + v;
    let w = v - (z - u);
    let t = z * z;
    let t1 = z - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
    let r = (z * t1) / (t1 - 2.0) - (w + z * w);
    z = 1.0 - (r - z);
    let mut j = hi(z);
    j = j.wrapping_add(n << 20);
    if (j >> 20) <= 0 {
        z = libm::scalbn(z, n);
    } else {
        z = with_hi(z, hi(z).wrapping_add(n << 20));
    }
    s * z
}

/// V8's `sinh`.
pub fn sinh(x: f64) -> f64 {
    const KSINH_OVERFLOW: f64 = 710.4758600739439;
    const TWO_M28: f64 = 3.725290298461914e-9;
    const LOG_MAXD: f64 = 709.7822265625;
    let h = if x < 0.0 { -0.5 } else { 0.5 };
    let ax = x.abs();
    if ax < 22.0 {
        if ax < TWO_M28 {
            return x;
        }
        let t = libm::expm1(ax);
        if ax < 1.0 {
            return h * (2.0 * t - t * t / (t + 1.0));
        }
        return h * (t + t / (t + 1.0));
    }
    if ax < LOG_MAXD {
        return h * exp(ax);
    }
    if ax <= KSINH_OVERFLOW {
        let w = exp(0.5 * ax);
        let t = h * w;
        return t * w;
    }
    x * 1.0e307
}

/// V8's `cosh`.
pub fn cosh(x: f64) -> f64 {
    const KCOSH_OVERFLOW: f64 = 710.4758600739439;
    let ix = hi(x) & 0x7fffffff;
    if ix < 0x3fd62e43 {
        let t = libm::expm1(x.abs());
        let w = 1.0 + t;
        if ix < 0x3c800000 {
            return w;
        }
        return 1.0 + (t * t) / (w + w);
    }
    if ix < 0x40360000 {
        let t = exp(x.abs());
        return 0.5 * t + 0.5 / t;
    }
    if ix < 0x40862e42 {
        return 0.5 * exp(x.abs());
    }
    if x.abs() <= KCOSH_OVERFLOW {
        let w = exp(0.5 * x.abs());
        let t = 0.5 * w;
        return t * w;
    }
    if ix >= 0x7ff00000 {
        return x * x;
    }
    f64::INFINITY
}

/// V8's `tanh`.
pub fn tanh(x: f64) -> f64 {
    let jx = hi(x);
    let ix = jx & 0x7fffffff;
    if ix >= 0x7ff00000 {
        return if jx >= 0 { 1.0 / x + 1.0 } else { 1.0 / x - 1.0 };
    }
    let z;
    if ix < 0x40360000 {
        if ix < 0x3e300000 {
            return x;
        }
        if ix >= 0x3ff00000 {
            let t = libm::expm1(2.0 * x.abs());
            z = 1.0 - 2.0 / (t + 2.0);
        } else {
            let t = libm::expm1(-2.0 * x.abs());
            z = -t / (t + 2.0);
        }
    } else {
        z = 1.0;
    }
    if jx >= 0 { z } else { -z }
}

/// V8's `asinh`.
pub fn asinh(x: f64) -> f64 {
    const LN2: f64 = 6.93147180559945286227e-01;
    let hx = hi(x);
    let ix = hx & 0x7fffffff;
    if ix >= 0x7ff00000 {
        return x + x;
    }
    if ix < 0x3e300000 {
        return x;
    }
    let w;
    if ix > 0x41b00000 {
        w = log(x.abs()) + LN2;
    } else if ix > 0x40000000 {
        let t = x.abs();
        w = log(2.0 * t + 1.0 / ((x * x + 1.0).sqrt() + t));
    } else {
        let t = x * x;
        w = log1p(x.abs() + t / (1.0 + (1.0 + t).sqrt()));
    }
    if hx > 0 { w } else { -w }
}

/// V8's `acosh`.
pub fn acosh(x: f64) -> f64 {
    const LN2: f64 = 6.93147180559945286227e-01;
    let hx = hi(x);
    let lx = lo(x);
    if hx < 0x3ff00000 {
        f64::NAN
    } else if hx >= 0x41b00000 {
        if hx >= 0x7ff00000 { x + x } else { log(x) + LN2 }
    } else if ((hx - 0x3ff00000) as u32 | lx) == 0 {
        0.0
    } else if hx > 0x40000000 {
        let t = x * x;
        log(2.0 * x - 1.0 / (x + (t - 1.0).sqrt()))
    } else {
        let t = x - 1.0;
        log1p(t + (2.0 * t + t * t).sqrt())
    }
}

/// V8's `atanh`.
pub fn atanh(x: f64) -> f64 {
    let hx = hi(x);
    let lx = lo(x);
    let ix = hx & 0x7fffffff;
    if (ix as u32 | ((lx | lx.wrapping_neg()) >> 31)) > 0x3ff00000 {
        return f64::NAN;
    }
    if ix == 0x3ff00000 {
        return if x > 0.0 { f64::INFINITY } else { f64::NEG_INFINITY };
    }
    if ix < 0x3e300000 {
        return x;
    }
    let x = with_hi(x, ix);
    let t = if ix < 0x3fe00000 {
        let t = x + x;
        0.5 * log1p(t + t * x / (1.0 - x))
    } else {
        0.5 * log1p((x + x) / (1.0 - x))
    };
    if hx >= 0 { t } else { -t }
}

pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

pub fn asin(x: f64) -> f64 {
    libm::asin(x)
}

pub fn acos(x: f64) -> f64 {
    libm::acos(x)
}

pub fn atan(x: f64) -> f64 {
    libm::atan(x)
}

pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

/// fdlibm `exp`; V8 returns `Math.E` for `exp(1)` (fdlibm's is one bit above it).
pub fn exp(x: f64) -> f64 {
    if x == 1.0 {
        return std::f64::consts::E;
    }
    libm::exp(x)
}

pub fn expm1(x: f64) -> f64 {
    libm::expm1(x)
}

pub fn log2(x: f64) -> f64 {
    libm::log2(x)
}
