// Black Hole Trash - a geodesic-traced Recycle Bin over the live desktop.
//
// Faithful WGSL port of s0xDk/ghostty-blackhole (MIT), itself after Eric
// Bruneton's "Real-time High-Quality Rendering of Non-Rotating Black Holes".
// Each pixel's null geodesic is integrated numerically in 3D - the Binet-form
// photon acceleration  a = -(3/2) h² x / r⁵  reproduces exact Schwarzschild
// bending. Everything falls out of that integration:
//
//   * shadow        - rays under b_crit = (3√3/2) r_s spiral into the horizon
//   * lensing       - escaped rays are projected back onto the desktop "sky"
//                     plane: your screen bends, magnifies, mirrors in the ring
//   * photon ring   - rays winding near the r = 1.5 r_s photon sphere
//   * accretion disk -  a thin tilted Keplerian disk the ray may cross several
//                     times (the far side arcs over and under the shadow);
//                     blackbody color from a Shakura-Sunyaev temperature
//                     profile, Doppler-shifted and beamed
//
// The terminal-specific modes (token/pomodoro/cursor decode) are replaced by
// a slow self-drift; the desktop capture plays the role of the lensed sky.
// A Kerr (spinning) mode integrates the exact rotating metric instead, see
// the Kerr section below.

// The disk "look" rides in the uniform buffer so the tray menu can switch
// presets live (values crossfaded on the CPU side).
struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    has_desktop: f32,
    temp: f32,          // hottest-annulus temperature, Kelvin
    incl: f32,          // disk inclination, rad (0 face-on, 1.57 edge-on)
    roll: f32,          // screen-plane rotation of the system
    inner: f32,         // disk inner edge, r_s
    outer: f32,         // disk outer edge, r_s
    opac: f32,          // near-disk opacity toward the background
    dopp: f32,          // Doppler mix (0 none, 1 full)
    beam: f32,          // beaming exponent
    gain: f32,          // disk emission brightness
    contr: f32,         // streak contrast
    wind: f32,          // spiral winding tightness
    speed: f32,         // streak speed; negative reverses orbit
    expo: f32,          // tonemap exposure
    star: f32,          // lensed starfield gain
    hole_radius: f32,   // shadow radius, fraction of screen height
    center_x: f32,      // hole centre in uv, computed on the CPU
    center_y: f32,      // (drift, pinned or follow-the-cursor placement)
    spin: f32,          // Kerr spin 0..0.99; 0 = Schwarzschild fast path
    _pad1: f32,
    _pad2: f32,
    cursor: vec4<f32>,
    cursor_motion: vec4<f32>,
    cursor_trail: array<vec4<f32>, 16>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var desktop_tex: texture_2d<f32>;
@group(0) @binding(2) var desktop_samp: sampler;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VSOut {
    var verts = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let xy = verts[vid];
    var out: VSOut;
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.uv  = vec2<f32>(xy.x * 0.5 + 0.5, 1.0 - (xy.y * 0.5 + 0.5));
    return out;
}

// ---------------------------------------------------------------- tunables --
// Disk look, size and centre come from the uniforms (tray menu / config
// file / placement gesture); only the hole-independent knobs stay compile-time.
const LENS_DEPTH: f32    = 13.0;   // hole-to-sky-plane distance in r_s - bigger = bends harder
const N_STEPS: i32       = 48;     // geodesic steps per pixel (perf dial)
const B_CRIT: f32        = 2.5980762; // critical impact parameter, r_s

// ------------------------------------------------------------------- helpers --
fn gmod(x: f32, y: f32) -> f32 { return x - y * floor(x / y); }

fn mirrorUV(uvin: vec2<f32>) -> vec2<f32> {
    let m = uvin - 2.0 * floor(uvin / 2.0);
    return 1.0 - abs(1.0 - m);
}

fn rot(v: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(c * v.x - s * v.y, s * v.x + c * v.y);
}

fn hash21(pin: vec2<f32>) -> f32 {
    var p = fract(pin * vec2<f32>(234.34, 435.345));
    p = p + dot(p, p + 34.23);
    return fract(p.x * p.y);
}

fn sd_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let ba = b - a;
    let h = clamp(dot(p - a, ba) / max(dot(ba, ba), 1e-4), 0.0, 1.0);
    return length(p - (a + ba * h));
}

fn cross2(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

fn sd_triangle(
    p: vec2<f32>,
    a: vec2<f32>,
    b: vec2<f32>,
    c: vec2<f32>,
) -> f32 {
    let d = min(sd_segment(p, a, b), min(sd_segment(p, b, c), sd_segment(p, c, a)));
    let s0 = cross2(b - a, p - a);
    let s1 = cross2(c - b, p - b);
    let s2 = cross2(a - c, p - c);
    let inside = (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0)
        || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0);
    return select(d, -d, inside);
}

fn sd_box(p: vec2<f32>, half_size: vec2<f32>) -> f32 {
    let q = abs(p) - half_size;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
}

fn cursor_arrow_sdf(
    fragment_px: vec2<f32>,
    origin_px: vec2<f32>,
    direction_px: vec2<f32>,
    stretch: f32,
    scale: f32,
) -> f32 {
    let fallback = vec2<f32>(1.0, 0.0);
    let tangent = select(fallback, normalize(direction_px), length(direction_px) > 0.05);
    let normal = vec2<f32>(-tangent.y, tangent.x);
    let rel = fragment_px - origin_px;
    var local = vec2<f32>(dot(rel, tangent), dot(rel, normal));
    local.x = local.x / max(stretch, 0.1);
    local = local / max(scale, 0.05);

    let head = sd_triangle(
        local,
        vec2<f32>(10.5, 0.0),
        vec2<f32>(-3.5, -6.2),
        vec2<f32>(-3.5, 6.2),
    );
    let stem = sd_box(local - vec2<f32>(-5.0, 0.0), vec2<f32>(4.2, 1.75));
    return min(head, stem) * scale;
}

fn cursor_overlay(base: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    if (u.cursor_motion.z < 0.5) {
        return base;
    }

    let resolution = max(u.resolution, vec2<f32>(1.0));
    let fragment_px = uv * resolution;
    let center_px = vec2<f32>(u.center_x, u.center_y) * resolution;
    let hole_px = max(u.hole_radius * resolution.y, 1.0);
    if (length(fragment_px - center_px) > hole_px * 6.0 + 48.0) {
        return base;
    }
    let absorption = smoothstep(0.0, 1.0, u.cursor.w);
    let collapse = smoothstep(0.18, 1.0, absorption);
    let strength = clamp(u.cursor.z, 0.0, 1.0);
    let approach = smoothstep(0.06, 0.82, strength);

    var trail_glow = 0.0;
    var trail_ink = 0.0;
    for (var i: i32 = 0; i < 15; i = i + 1) {
        let ia = u32(i);
        let ib = u32(i + 1);
        let sa = u.cursor_trail[ia];
        let sb = u.cursor_trail[ib];
        var a = sa.xy * resolution;
        var b = sb.xy * resolution;
        let history = f32(i) / 14.0;
        let spiral_a = approach * history * 0.95
            + collapse * (3.8 + history * 1.1);
        let spiral_b = approach * (history + 1.0 / 14.0) * 0.95
            + collapse * (3.8 + (history + 1.0 / 14.0) * 1.1);
        a = center_px + rot(a - center_px, spiral_a);
        b = center_px + rot(b - center_px, spiral_b);
        let radial_collapse = collapse * collapse;
        let approach_a = approach * history * 0.18;
        let approach_b = approach * (history + 1.0 / 14.0) * 0.18;
        a = mix(
            a,
            center_px,
            clamp(approach_a + radial_collapse * mix(0.92, 0.56, history), 0.0, 1.0),
        );
        b = mix(
            b,
            center_px,
            clamp(approach_b + radial_collapse * mix(0.90, 0.52, history), 0.0, 1.0),
        );
        let distance = sd_segment(fragment_px, a, b);
        let width = mix(1.2, 7.5, history) * mix(0.75, 1.35, strength);
        let age = 0.5 * (sa.z + sb.z);
        let age_fade = 1.0 - smoothstep(0.10, 0.38, age);
        let history_fade = 1.0 - smoothstep(0.25, 1.0, history);
        let valid = min(sa.w, sb.w);
        let weight = age_fade * history_fade * valid;
        trail_glow = trail_glow
            + exp2(-distance * distance / max(width * width, 0.5)) * weight;
        trail_ink = trail_ink
            + (1.0 - smoothstep(0.4, width * 0.48 + 0.7, distance)) * weight;
    }

    let luminance = dot(base, vec3<f32>(0.2126, 0.7152, 0.0722));
    let ink = select(vec3<f32>(0.055, 0.06, 0.07), vec3<f32>(0.94, 0.95, 0.97), luminance < 0.42);
    let rim = select(vec3<f32>(0.96, 0.90, 0.69), vec3<f32>(0.10, 0.075, 0.035), luminance < 0.42);

    var color = mix(base, ink, clamp(trail_ink * 0.075, 0.0, 0.22));
    color = color + vec3<f32>(1.0, 0.74, 0.30) * min(trail_glow * 0.035, 0.13);

    for (var ghost: i32 = 0; ghost < 4; ghost = ghost + 1) {
        let index = u32(2 + ghost * 3);
        let newer = u.cursor_trail[index - 1u];
        let sample = u.cursor_trail[index];
        let older = u.cursor_trail[min(index + 1u, 15u)];
        var origin = sample.xy * resolution;
        let ghost_history = f32(ghost) / 3.0;
        let ghost_spiral = approach * ghost_history * 0.95
            + collapse * (3.8 + ghost_history * 1.1);
        origin = center_px + rot(origin - center_px, ghost_spiral);
        origin = mix(
            origin,
            center_px,
            clamp(
                approach * ghost_history * 0.18
                    + collapse * collapse * mix(0.88, 0.54, ghost_history),
                0.0,
                1.0,
            ),
        );
        let direction = rot((newer.xy - older.xy) * resolution, ghost_spiral);
        let ghost_scale = (1.0 - absorption * 0.58) * mix(0.94, 0.68, f32(ghost) / 3.0);
        let ghost_stretch = mix(1.0, 1.75, strength) * (1.0 + f32(ghost) * 0.10);
        let sdf = cursor_arrow_sdf(
            fragment_px,
            origin,
            direction,
            ghost_stretch,
            ghost_scale,
        );
        let mask = (1.0 - smoothstep(-0.25, 1.6 + f32(ghost) * 0.35, sdf))
            * (1.0 - smoothstep(0.06, 0.34, sample.z))
            * sample.w
            * mix(0.30, 0.09, f32(ghost) / 3.0);
        color = mix(color, mix(ink, rim, 0.22), mask);
    }

    var head_origin = u.cursor.xy * resolution;
    let head_spiral = collapse * 3.8;
    head_origin = center_px + rot(head_origin - center_px, head_spiral);
    head_origin = mix(head_origin, center_px, collapse * collapse);
    var head_direction = rot(u.cursor_motion.xy * resolution, head_spiral);
    if (length(head_direction) <= 0.05) {
        head_direction = center_px - head_origin;
    }
    let head_scale = max(1.0 - smoothstep(0.72, 1.0, absorption), 0.03);
    let head_stretch = mix(1.0, 2.1, strength);
    let head_sdf = cursor_arrow_sdf(
        fragment_px,
        head_origin,
        head_direction,
        head_stretch,
        head_scale,
    );
    let outline = 1.0 - smoothstep(0.15, 1.55, head_sdf);
    let fill = 1.0 - smoothstep(-0.95, 0.25, head_sdf);
    color = mix(color, rim, outline * (1.0 - absorption));
    color = mix(color, ink, fill * (1.0 - absorption * 0.75));

    let horizon_mask = smoothstep(hole_px * 0.58, hole_px * 0.94, length(fragment_px - center_px));
    return mix(base, color, horizon_mask);
}

// value noise whose y lattice wraps every perY cells - the disk's angular
// dimension, so streaks tile seamlessly across the atan branch cut
fn vnoiseWrapY(p: vec2<f32>, perY: f32) -> f32 {
    let i = floor(p);
    var f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    let y0 = gmod(i.y, perY);
    let y1 = gmod(i.y + 1.0, perY);
    return mix(mix(hash21(vec2<f32>(i.x, y0)),       hash21(vec2<f32>(i.x + 1.0, y0)), f.x),
               mix(hash21(vec2<f32>(i.x, y1)),       hash21(vec2<f32>(i.x + 1.0, y1)), f.x),
               f.y);
}

// blackbody color from temperature in Kelvin (Tanner Helland fit, normalized)
fn blackbody(T: f32) -> vec3<f32> {
    let t = clamp(T, 1500.0, 40000.0) / 100.0;
    var r = 1.0;
    if (t > 66.0) { r = clamp(1.292936 * pow(t - 60.0, -0.1332047), 0.0, 1.0); }
    var g = 0.0;
    if (t <= 66.0) { g = clamp(0.3900816 * log(t) - 0.6318414, 0.0, 1.0); }
    else           { g = clamp(1.1298909 * pow(t - 60.0, -0.0755148), 0.0, 1.0); }
    var b = 1.0;
    if (t < 66.0) {
        if (t <= 19.0) { b = 0.0; }
        else { b = clamp(0.5432068 * log(t - 10.0) - 1.1962540, 0.0, 1.0); }
    }
    return vec3<f32>(r, g, b);
}

// sparse procedural starfield indexed by (bent) ray direction
fn stars(d: vec3<f32>) -> vec3<f32> {
    let sph = vec2<f32>(atan2(d.x, -d.z), asin(clamp(d.y, -1.0, 1.0)));
    let g   = sph * 40.0;
    let id  = floor(g);
    let h   = hash21(id);
    if (h < 0.92) { return vec3<f32>(0.0); }
    let f   = fract(g) - 0.5;
    let off = (vec2<f32>(hash21(id + 17.3), hash21(id + 31.7)) - 0.5) * 0.7;
    let spark = smoothstep(0.10, 0.0, length(f - off));
    let tw    = 0.7 + 0.3 * sin(u.time * (0.5 + 2.0 * hash21(id + 5.1)) + 40.0 * h);
    let tint  = mix(vec3<f32>(1.0, 0.82, 0.60), vec3<f32>(0.75, 0.85, 1.0), hash21(id + 2.9));
    return tint * spark * tw * ((h - 0.92) / 0.08);
}

// live desktop when available; procedural test pattern otherwise
fn background(uvin: vec2<f32>) -> vec3<f32> {
    let uvm = mirrorUV(uvin);
    if (u.has_desktop > 0.5) {
        return textureSampleLevel(desktop_tex, desktop_samp, uvm, 0.0).rgb;
    }
    let grad = vec3<f32>(uvm.x, uvm.y, 0.6);
    let cx = step(0.5, fract(uvm.x * 16.0));
    let cy = step(0.5, fract(uvm.y * 9.0));
    let c  = abs(cx - cy);
    let checker = mix(vec3<f32>(0.05, 0.06, 0.09), vec3<f32>(0.14, 0.16, 0.22), c);
    return mix(checker, grad, 0.35);
}

// Shade one thin-disk crossing. xc/vdir in world space; returns the
// transmittance-weighted emission in rgb and the local density in a.
fn shade_crossing(
    xc: vec3<f32>, vdir: vec3<f32>, n: vec3<f32>, e2: vec3<f32>,
    rin: f32, rout: f32, t: f32, sdir: f32, spd: f32, trans: f32,
) -> vec4<f32> {
    let rc = length(xc);
    if (rc <= rin || rc >= rout) {
        return vec4<f32>(0.0);
    }
    let band = smoothstep(rin, rin * 1.25, rc)
             * (1.0 - smoothstep(rout * 0.70, rout, rc));

    // disk-plane polar coords for the streak texture
    let phi   = atan2(dot(xc, e2), xc.x);
    let turns = phi / 6.2831853;
    let kep   = pow(rin / rc, 1.5);
    // sqrt(1 - 1.5/r): time runs slower for the inner orbits
    let gloc  = sqrt(max(1.0 - 1.5 / rc, 0.02));
    let swirl = rc * u.wind * 0.12 - t * kep * spd * gloc * sdir;
    var streaks = vnoiseWrapY(vec2<f32>(rc * 2.8, turns * 19.0 + swirl * 3.0), 19.0) * 0.65
                + vnoiseWrapY(vec2<f32>(rc * 1.0, turns * 9.0  + swirl * 1.5 + 7.0), 9.0) * 0.35;
    streaks = 0.35 + u.contr * streaks * streaks;

    // relativistic Doppler + gravitational shift for circular-orbit gas
    let gasdir = normalize(cross(n, xc)) * sdir;
    let beta   = clamp(inverseSqrt(max(2.0 * (rc - 1.0), 0.2)), 0.0, 0.99);
    var g = gloc / max(1.0 + beta * dot(gasdir, vdir), 0.05);
    g = mix(1.0, g, u.dopp);

    // Shakura-Sunyaev temperature profile, peak normalized to 1
    let xpr   = max(1.0 - sqrt(rin / rc), 0.0);
    let tprof = pow(rin / rc, 0.75) * pow(xpr, 0.25) / 0.488;
    let cbb   = blackbody(u.temp * tprof * g);
    let boost = pow(g, u.beam);

    let density = band * streaks;
    return vec4<f32>(trans * cbb * (u.gain * 2.2 * density * tprof * tprof * boost), density);
}

// -------------------------- Kerr (spinning) mode ---------------------------
// Exact Kerr null geodesics via the Hamiltonian in Kerr-Schild coordinates:
// H = 1/2 (|p|^2 - E^2) - 1/2 f (l.p + E)^2, with f = 2 M r^3/(r^4 + a^2 z^2).
// M = 0.5 so r_s = 2M = 1 keeps every existing unit; the user-facing spin
// 0..1 maps to a = 0.5 spin (extremal at a = M). dx/dlambda is analytic and
// dp/dlambda comes from central differences of H, which avoids hand-deriving
// Christoffel symbols while integrating the exact metric.
const KERR_STEPS: i32 = 88;

// Prograde ISCO radius in r_s units (Bardeen 1972). spin = a/M in [0,1).
// At spin 0 this is 3 r_s (= 6M), shrinking toward ~0.6 r_s as spin -> 1:
// a fast-spinning hole lets the disk hug the shadow, which is the most
// visible consequence of spin.
// Dramatized frame-drag swirl. The exact azimuthal twist of Kerr lensing
// is physically real but imperceptible at desktop scale, so the VISIBLE
// layer exaggerates it: background samples rotate around the hole, strongest
// near the ring, prograde with the disk. Zero spin is an exact no-op.
fn drag_twist(b_rs: f32, spin_signed: f32) -> f32 {
    return 1.3 * spin_signed / (1.0 + 0.8 * pow(b_rs / B_CRIT, 2.0));
}

fn kerr_isco_rs(spin: f32) -> f32 {
    let a = clamp(spin, 0.0, 0.999);
    let z1 = 1.0 + pow(1.0 - a * a, 1.0 / 3.0)
           * (pow(1.0 + a, 1.0 / 3.0) + pow(1.0 - a, 1.0 / 3.0));
    let z2 = sqrt(3.0 * a * a + z1 * z1);
    let r_m = 3.0 + z2 - sqrt((3.0 - z1) * (3.0 + z1 + 2.0 * z2));
    return r_m * 0.5; // M units -> r_s units (r_s = 2M)
}

fn ks_r(pos: vec3<f32>, a: f32) -> f32 {
    let rho2 = dot(pos, pos);
    let bq = rho2 - a * a;
    let r2 = 0.5 * (bq + sqrt(bq * bq + 4.0 * a * a * pos.z * pos.z));
    return sqrt(max(r2, 1e-8));
}

fn kerr_h(pos: vec3<f32>, pm: vec3<f32>, a: f32) -> f32 {
    let r = ks_r(pos, a);
    let r2 = r * r;
    let den = r2 + a * a;
    let l = vec3<f32>(
        (r * pos.x + a * pos.y) / den,
        (r * pos.y - a * pos.x) / den,
        pos.z / max(r, 1e-6),
    );
    let f = r2 * r / (r2 * r2 + a * a * pos.z * pos.z); // 2 M r^3 / (...), M = 0.5
    let lp = dot(l, pm) + 1.0; // l^mu p_mu with E = 1 (outgoing KS; exact Kerr outside the horizon)
    return 0.5 * (dot(pm, pm) - 1.0) - 0.5 * f * lp * lp;
}

fn kerr_dxdl(pos: vec3<f32>, pm: vec3<f32>, a: f32) -> vec3<f32> {
    let r = ks_r(pos, a);
    let r2 = r * r;
    let den = r2 + a * a;
    let l = vec3<f32>(
        (r * pos.x + a * pos.y) / den,
        (r * pos.y - a * pos.x) / den,
        pos.z / max(r, 1e-6),
    );
    let f = r2 * r / (r2 * r2 + a * a * pos.z * pos.z);
    let lp = dot(l, pm) + 1.0; // matches kerr_h
    return pm - f * lp * l;
}

fn kerr_dpdl(pos: vec3<f32>, pm: vec3<f32>, a: f32) -> vec3<f32> {
    let e = 2e-3;
    return vec3<f32>(
        kerr_h(pos - vec3<f32>(e, 0.0, 0.0), pm, a) - kerr_h(pos + vec3<f32>(e, 0.0, 0.0), pm, a),
        kerr_h(pos - vec3<f32>(0.0, e, 0.0), pm, a) - kerr_h(pos + vec3<f32>(0.0, e, 0.0), pm, a),
        kerr_h(pos - vec3<f32>(0.0, 0.0, e), pm, a) - kerr_h(pos + vec3<f32>(0.0, 0.0, e), pm, a),
    ) / (2.0 * e);
}

// ---------------------------------------------------------------------- main --
@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let uv     = in.uv;
    let aspect = u.resolution.x / max(u.resolution.y, 1.0);
    let t      = u.time;

    let rin  = max(u.inner, 1.6);
    let rout = max(u.outer, rin + 0.5);
    let rh   = u.hole_radius;            // shadow radius in screen units

    // hole centre computed on the CPU (drift / pinned / follow-the-cursor)
    let center = vec2<f32>(u.center_x, u.center_y);

    // aspect-corrected frame centered on the hole (y in screen heights)
    let p    = (uv - center) * vec2<f32>(aspect, 1.0);
    let plen = length(p);

    // screen <-> world: shadow's angular size is B_CRIT r_s at rh screen units
    let W  = B_CRIT / max(rh, 1e-4);
    let pr = rot(vec2<f32>(p.x, -p.y), u.roll) * W;
    let b  = length(pr);                 // impact parameter, r_s units

    // fade real 1/b lensing a few disk diameters out so the whole screen
    // doesn't shimmer as the hole drifts (deliberately unphysical)
    let window = exp(-pow(plen / (7.0 * rh), 2.0));

    let bmax = rout + 3.0;               // rays beyond this can't touch the disk
    let Z0   = max(14.0, rout + 5.0);    // camera distance (shared with tracer)

    // ================= far field: analytic weak deflection ==================
    // finite-camera fitted mapping - sub-1% displacement match at the handoff
    // circle, so there is no visible seam against the integrated region
    if (b >= bmax) {
        let uu   = Z0 * inverseSqrt(Z0 * Z0 + b * b);
        let defl = (2.0 / (W * W)) / max(plen, 1e-4)
                 * (1.29 * uu + 0.07) * max(LENS_DEPTH - 2.14 * uu + 0.75, 0.0)
                 * window;
        let dir = p / max(plen, 1e-5);
        // mild chromatic aberration: blue bends a touch more than red
        let ab = 0.035 * smoothstep(1.0, 2.0, b / bmax);
        let sdir_far = select(1.0, -1.0, u.speed < 0.0);
        let tw = drag_twist(b, u.spin * sdir_far);
        let sp_r = rot(p - dir * defl * (1.0 - ab), tw);
        let sp_g = rot(p - dir * defl, tw);
        let sp_b = rot(p - dir * defl * (1.0 + ab), tw);
        let col = vec3<f32>(
            background(center + sp_r / vec2<f32>(aspect, 1.0)).r,
            background(center + sp_g / vec2<f32>(aspect, 1.0)).g,
            background(center + sp_b / vec2<f32>(aspect, 1.0)).b,
        );
        let d = normalize(vec3<f32>(-(pr / b) * (2.0 / b), -1.0));
        return vec4<f32>(cursor_overlay(col + stars(d) * u.star * window, uv), 1.0);
    }

    // ====================== near field: trace the geodesic ==================
    // Parallel rays from a distant camera at +z; hole at origin, r_s = 1.
    var x = vec3<f32>(pr, Z0);
    var v = vec3<f32>(0.0, 0.0, -1.0);
    let h2 = dot(pr, pr);                // conserved angular momentum²

    // disk plane: normal tilted DISK_INCL about the screen x-axis
    let ci = cos(u.incl);
    let si = sin(u.incl);
    let n  = vec3<f32>(0.0, si, ci);
    let e2 = vec3<f32>(0.0, ci, -si);    // in-plane axis completing (x̂, e2, n)
    let sdir = select(1.0, -1.0, u.speed < 0.0);
    let spd  = abs(u.speed);

    var emitc = vec3<f32>(0.0);          // accumulated disk light (HDR)
    var trans = 1.0;                     // transmittance toward the background
    var captured = false;

    if (u.spin < 0.005) {
        // -------- Schwarzschild fast path (non-spinning) --------
        var sPrev = dot(x, n);
        var xPrev = x;
        for (var i: i32 = 0; i < N_STEPS; i = i + 1) {
            var r2 = dot(x, x);
            if (r2 < 1.0) { captured = true; break; }     // through the horizon
            if (x.z < -Z0 && v.z < 0.0) { break; }        // escaped out the back
            if (r2 > 4.0 * Z0 * Z0) { break; }            // flung far sideways
            var r = sqrt(r2);
            // step scales with radius: fine near the photon sphere
            let dt = clamp(0.16 * r, 0.03, 1.5);
            // leapfrog (kick-drift-kick) keeps near-critical orbits stable
            var acc = -1.5 * h2 * x / (r2 * r2 * r);
            v = v + acc * (0.5 * dt);
            x = x + v * dt;
            r2 = dot(x, x);
            r  = sqrt(r2);
            acc = -1.5 * h2 * x / (r2 * r2 * r);
            v = v + acc * (0.5 * dt);

            // thin-disk crossing
            let s = dot(x, n);
            if (s * sPrev < 0.0 && trans > 0.02) {
                let tc = sPrev / (sPrev - s);
                let xc = mix(xPrev, x, tc);
                let sh = shade_crossing(xc, normalize(v), n, e2, rin, rout, t, sdir, spd, trans);
                emitc = emitc + sh.rgb;
                trans = trans * (1.0 - clamp(u.opac * sh.a, 0.0, 1.0));
            }
            sPrev = s;
            xPrev = x;
        }
        // rays still winding when the budget ran out are as good as captured
        if (!captured && dot(x, x) < 4.0) { captured = true; }
    } else {
        // -------- Kerr path: exact spinning-hole geodesics --------
        // Integrate in the black hole frame where the spin axis is +z (the
        // disk normal): rows of the world->BH rotation are (x_hat, e2, n).
        let a = 0.5 * u.spin * sdir;     // a = M spin, prograde with the disk
        let rhor = 0.5 + sqrt(max(0.25 - a * a, 0.0)); // event horizon radius
        // spin shrinks the ISCO, so the disk reaches much closer to the
        // hole: scale the preset inner edge by the ISCO ratio (ISCO = 3 r_s
        // at zero spin) and keep it just outside the horizon
        let rin_k = max(rin * kerr_isco_rs(u.spin) / 3.0, kerr_isco_rs(u.spin));
        var xb = vec3<f32>(x.x, dot(x, e2), dot(x, n));
        var pb = vec3<f32>(v.x, dot(v, e2), dot(v, n));
        var sPrev = xb.z;
        var xPrev = xb;
        for (var i: i32 = 0; i < KERR_STEPS; i = i + 1) {
            let r = ks_r(xb, a);
            if (r < rhor + 0.02) { captured = true; break; }
            if (r > 1.4 * Z0 && dot(xb, kerr_dxdl(xb, pb, a)) > 0.0) { break; }
            // leapfrog in Hamiltonian form; dp from central differences of H
            let dl = clamp(0.16 * r, 0.02, 1.1);
            pb = pb + kerr_dpdl(xb, pb, a) * (0.5 * dl);
            xb = xb + kerr_dxdl(xb, pb, a) * dl;
            pb = pb + kerr_dpdl(xb, pb, a) * (0.5 * dl);

            // thin-disk crossing at the BH equator (z = 0 in this frame)
            if (xb.z * sPrev < 0.0 && trans > 0.02) {
                let tc = sPrev / (sPrev - xb.z);
                let xcb = mix(xPrev, xb, tc);
                // back to world space for the shared disk shading
                let xc = xcb.x * vec3<f32>(1.0, 0.0, 0.0) + xcb.y * e2 + xcb.z * n;
                let vb = kerr_dxdl(xb, pb, a);
                let vw = vb.x * vec3<f32>(1.0, 0.0, 0.0) + vb.y * e2 + vb.z * n;
                let sh = shade_crossing(xc, normalize(vw), n, e2, rin, rout, t, sdir, spd, trans);
                emitc = emitc + sh.rgb;
                trans = trans * (1.0 - clamp(u.opac * sh.a, 0.0, 1.0));
            }
            sPrev = xb.z;
            xPrev = xb;
        }
        // budget exhausted while still winding near the photon orbits
        // (Kerr retrograde photon orbit reaches r = 4M = 2.0; add margin)
        if (!captured && ks_r(xb, a) < 2.4) { captured = true; }
        // back to world space for the shared background projection
        let vb = kerr_dxdl(xb, pb, a);
        x = xb.x * vec3<f32>(1.0, 0.0, 0.0) + xb.y * e2 + xb.z * n;
        v = vb.x * vec3<f32>(1.0, 0.0, 0.0) + vb.y * e2 + vb.z * n;
    }

    // ---- background: where did the escaped ray come from? ----
    var bg = vec3<f32>(0.0);
    if (!captured) {
        let d = normalize(v);
        bg = bg + stars(d) * u.star * window;
        if (d.z < -0.05) {
            // project the exit ray onto the desktop sky plane at z = -LENS_DEPTH
            let tpl = (-LENS_DEPTH - x.z) / d.z;
            let hp  = x + d * tpl;
            let q   = rot(hp.xy, -u.roll) / W;
            var sp  = vec2<f32>(q.x, -q.y);
            sp = rot(sp, drag_twist(b, u.spin * sdir));
            // fade the *displacement*, never the color - no seam anywhere
            let suv = center + (p + (sp - p) * window) / vec2<f32>(aspect, 1.0);
            // rays bent past ~90° never reach the sky plane; fade them out
            let toward = smoothstep(0.05, 0.35, -d.z);
            bg = bg + background(suv) * toward;
        }
    }

    // disk light is HDR; tonemap it on top of the (untouched) desktop sample
    let col = bg * trans + (vec3<f32>(1.0) - exp(-emitc * u.expo));
    return vec4<f32>(cursor_overlay(col, uv), 1.0);
}
