#include "cpp_accel_common.h"

enum ValueType : uint8_t {
    V_INT = 1,
    V_FLOAT = 2,
    V_STRING = 3,
    V_BOOL = 4,
    V_NONE = 5,
};

struct PyValue {
    uint8_t typ;
    int64_t i;
    double f;
    uint8_t b;
    char s[256];
};

struct PyVar {
    uint8_t used;
    char key[64];
    PyValue v;
};

struct PyInterp {
    uint8_t used;
    PyVar vars[128];
};

static PyInterp g_interps[16];

static inline size_t c_len(const char* s) {
    size_t n = 0;
    while (s[n] != '\0') ++n;
    return n;
}

static inline int c_eq(const char* a, const char* b) {
    size_t i = 0;
    while (a[i] != '\0' && b[i] != '\0') {
        if (a[i] != b[i]) return 0;
        ++i;
    }
    return a[i] == '\0' && b[i] == '\0';
}

static inline void trim_span(const char* in, size_t len, size_t* out_l, size_t* out_r) {
    size_t l = 0;
    size_t r = len;
    while (l < r && (in[l] == ' ' || in[l] == '\t' || in[l] == '\n' || in[l] == '\r')) ++l;
    while (r > l && (in[r - 1] == ' ' || in[r - 1] == '\t' || in[r - 1] == '\n' || in[r - 1] == '\r')) --r;
    *out_l = l;
    *out_r = r;
}

static inline void copy_trimmed(const char* in, size_t len, char* out, size_t out_cap) {
    size_t l, r;
    trim_span(in, len, &l, &r);
    size_t n = r > l ? (r - l) : 0;
    if (n + 1 > out_cap) n = out_cap - 1;
    for (size_t i = 0; i < n; ++i) out[i] = in[l + i];
    out[n] = '\0';
}

static inline void set_str(PyValue* v, const char* s) {
    v->typ = V_STRING;
    size_t n = c_len(s);
    if (n > 255) n = 255;
    for (size_t i = 0; i < n; ++i) v->s[i] = s[i];
    v->s[n] = '\0';
}

static inline void value_to_str(const PyValue* v, char* out, size_t out_cap) {
    if (out_cap == 0) return;
    out[0] = '\0';

    if (v->typ == V_INT) {
        char tmp[32];
        size_t n = 0;
        int64_t x = v->i;
        uint8_t neg = 0;
        if (x == 0) {
            out[0] = '0'; out[1] = '\0';
            return;
        }
        if (x < 0) { neg = 1; x = -x; }
        while (x > 0 && n < sizeof(tmp)) {
            tmp[n++] = static_cast<char>('0' + (x % 10));
            x /= 10;
        }
        size_t p = 0;
        if (neg && p + 1 < out_cap) out[p++] = '-';
        while (n > 0 && p + 1 < out_cap) out[p++] = tmp[--n];
        out[p] = '\0';
        return;
    }

    if (v->typ == V_FLOAT) {
        double x = v->f;
        uint8_t neg = 0;
        if (x < 0.0) { neg = 1; x = -x; }
        int64_t whole = static_cast<int64_t>(x);
        int64_t frac = static_cast<int64_t>((x - static_cast<double>(whole)) * 1000000.0 + 0.5);
        PyValue iv; iv.typ = V_INT; iv.i = whole;
        char a[64]; value_to_str(&iv, a, sizeof(a));

        size_t p = 0;
        if (neg && p + 1 < out_cap) out[p++] = '-';
        for (size_t i = 0; a[i] != '\0' && p + 1 < out_cap; ++i) out[p++] = a[i];
        if (p + 1 < out_cap) out[p++] = '.';

        char fracbuf[7];
        for (int i = 5; i >= 0; --i) {
            fracbuf[i] = static_cast<char>('0' + (frac % 10));
            frac /= 10;
        }
        int end = 5;
        while (end > 0 && fracbuf[end] == '0') --end;
        for (int i = 0; i <= end && p + 1 < out_cap; ++i) out[p++] = fracbuf[i];
        out[p] = '\0';
        return;
    }

    if (v->typ == V_BOOL) {
        const char* t = v->b ? "True" : "False";
        size_t i = 0;
        while (t[i] != '\0' && i + 1 < out_cap) { out[i] = t[i]; ++i; }
        out[i] = '\0';
        return;
    }

    if (v->typ == V_NONE) {
        const char* t = "None";
        size_t i = 0;
        while (t[i] != '\0' && i + 1 < out_cap) { out[i] = t[i]; ++i; }
        out[i] = '\0';
        return;
    }

    if (v->typ == V_STRING) {
        size_t i = 0;
        while (v->s[i] != '\0' && i + 1 < out_cap) { out[i] = v->s[i]; ++i; }
        out[i] = '\0';
        return;
    }
}

static inline PyInterp* get_interp(uint32_t id) {
    if (id == 0 || id > 16) return nullptr;
    PyInterp* p = &g_interps[id - 1];
    if (!p->used) return nullptr;
    return p;
}

static inline PyVar* find_var(PyInterp* p, const char* name) {
    for (size_t i = 0; i < 128; ++i) {
        if (p->vars[i].used && c_eq(p->vars[i].key, name)) return &p->vars[i];
    }
    return nullptr;
}

static inline PyVar* ensure_var(PyInterp* p, const char* name) {
    PyVar* existing = find_var(p, name);
    if (existing) return existing;
    for (size_t i = 0; i < 128; ++i) {
        if (!p->vars[i].used) {
            p->vars[i].used = 1;
            size_t n = c_len(name);
            if (n > 63) n = 63;
            for (size_t k = 0; k < n; ++k) p->vars[i].key[k] = name[k];
            p->vars[i].key[n] = '\0';
            p->vars[i].v.typ = V_NONE;
            return &p->vars[i];
        }
    }
    return nullptr;
}

static int parse_int64(const char* s, int64_t* out) {
    size_t i = 0;
    int neg = 0;
    if (s[0] == '-') { neg = 1; i = 1; }
    if (s[i] == '\0') return 0;
    int64_t v = 0;
    for (; s[i] != '\0'; ++i) {
        if (s[i] < '0' || s[i] > '9') return 0;
        v = v * 10 + static_cast<int64_t>(s[i] - '0');
    }
    *out = neg ? -v : v;
    return 1;
}

static int parse_float64(const char* s, double* out) {
    // simple parser
    size_t i = 0;
    int neg = 0;
    if (s[0] == '-') { neg = 1; i = 1; }
    if (s[i] == '\0') return 0;

    double whole = 0.0;
    int saw_digit = 0;
    for (; s[i] >= '0' && s[i] <= '9'; ++i) {
        saw_digit = 1;
        whole = whole * 10.0 + static_cast<double>(s[i] - '0');
    }

    double frac = 0.0;
    double base = 1.0;
    if (s[i] == '.') {
        ++i;
        for (; s[i] >= '0' && s[i] <= '9'; ++i) {
            saw_digit = 1;
            frac = frac * 10.0 + static_cast<double>(s[i] - '0');
            base *= 10.0;
        }
    }

    if (!saw_digit || s[i] != '\0') return 0;
    double v = whole + frac / base;
    *out = neg ? -v : v;
    return 1;
}

static int eval_expr(PyInterp* p, const char* expr, PyValue* out, char* err, size_t err_cap);

static int apply_arith(const PyValue* a, const PyValue* b, char op, PyValue* out, char* err, size_t err_cap) {
    if (a->typ == V_STRING && b->typ == V_STRING && op == '+') {
        out->typ = V_STRING;
        size_t pa = 0;
        while (a->s[pa] != '\0' && pa < 255) { out->s[pa] = a->s[pa]; ++pa; }
        size_t pb = 0;
        while (b->s[pb] != '\0' && pa < 255) { out->s[pa++] = b->s[pb++]; }
        out->s[pa] = '\0';
        return 1;
    }

    double da;
    double db;

    if (a->typ == V_INT) da = static_cast<double>(a->i);
    else if (a->typ == V_FLOAT) da = a->f;
    else { set_str(reinterpret_cast<PyValue*>(err), ""); return 0; }

    if (b->typ == V_INT) db = static_cast<double>(b->i);
    else if (b->typ == V_FLOAT) db = b->f;
    else { return 0; }

    if (op == '/' && db == 0.0) {
        const char* m = "division by zero";
        size_t i = 0;
        while (m[i] != '\0' && i + 1 < err_cap) { err[i] = m[i]; ++i; }
        err[i] = '\0';
        return -1;
    }

    double r = 0.0;
    if (op == '+') r = da + db;
    else if (op == '-') r = da - db;
    else if (op == '*') r = da * db;
    else if (op == '/') r = da / db;
    else return 0;

    if (a->typ == V_INT && b->typ == V_INT && op != '/') {
        out->typ = V_INT;
        out->i = static_cast<int64_t>(r);
    } else {
        out->typ = V_FLOAT;
        out->f = r;
    }
    return 1;
}

static int eval_expr(PyInterp* p, const char* expr, PyValue* out, char* err, size_t err_cap) {
    char t[512];
    copy_trimmed(expr, c_len(expr), t, sizeof(t));

    size_t n = c_len(t);
    if (n == 0) {
        out->typ = V_NONE;
        return 1;
    }

    if ((t[0] == '"' && n >= 2 && t[n - 1] == '"') || (t[0] == '\'' && n >= 2 && t[n - 1] == '\'')) {
        out->typ = V_STRING;
        size_t m = n - 2;
        if (m > 255) m = 255;
        for (size_t i = 0; i < m; ++i) out->s[i] = t[i + 1];
        out->s[m] = '\0';
        return 1;
    }

    if (c_eq(t, "True")) { out->typ = V_BOOL; out->b = 1; return 1; }
    if (c_eq(t, "False")) { out->typ = V_BOOL; out->b = 0; return 1; }
    if (c_eq(t, "None")) { out->typ = V_NONE; return 1; }

    int64_t iv;
    if (parse_int64(t, &iv)) { out->typ = V_INT; out->i = iv; return 1; }

    double fv;
    if (parse_float64(t, &fv)) { out->typ = V_FLOAT; out->f = fv; return 1; }

    // builtins
    if (n > 4 && t[0]=='l' && t[1]=='e' && t[2]=='n' && t[3]=='(' && t[n-1]==')') {
        char arg[512];
        size_t m = n - 5;
        if (m > sizeof(arg)-1) m = sizeof(arg)-1;
        for (size_t i = 0; i < m; ++i) arg[i] = t[4 + i];
        arg[m] = '\0';

        PyValue av;
        if (!eval_expr(p, arg, &av, err, err_cap)) return 0;
        if (av.typ == V_STRING) {
            out->typ = V_INT;
            out->i = static_cast<int64_t>(c_len(av.s));
            return 1;
        }
        const char* m2 = "len() requires string";
        size_t k = 0;
        while (m2[k] != '\0' && k + 1 < err_cap) { err[k] = m2[k]; ++k; }
        err[k] = '\0';
        return 0;
    }

    if (n > 4 && t[0]=='s' && t[1]=='t' && t[2]=='r' && t[3]=='(' && t[n-1]==')') {
        char arg[512];
        size_t m = n - 5;
        if (m > sizeof(arg)-1) m = sizeof(arg)-1;
        for (size_t i = 0; i < m; ++i) arg[i] = t[4 + i];
        arg[m] = '\0';
        PyValue av;
        if (!eval_expr(p, arg, &av, err, err_cap)) return 0;
        char tmp[256];
        value_to_str(&av, tmp, sizeof(tmp));
        set_str(out, tmp);
        return 1;
    }

    if (n > 4 && t[0]=='i' && t[1]=='n' && t[2]=='t' && t[3]=='(' && t[n-1]==')') {
        char arg[512];
        size_t m = n - 5;
        if (m > sizeof(arg)-1) m = sizeof(arg)-1;
        for (size_t i = 0; i < m; ++i) arg[i] = t[4 + i];
        arg[m] = '\0';
        PyValue av;
        if (!eval_expr(p, arg, &av, err, err_cap)) return 0;
        if (av.typ == V_INT) { *out = av; return 1; }
        if (av.typ == V_FLOAT) { out->typ = V_INT; out->i = static_cast<int64_t>(av.f); return 1; }
        if (av.typ == V_STRING) {
            int64_t x;
            if (parse_int64(av.s, &x)) { out->typ = V_INT; out->i = x; return 1; }
            const char* m2 = "invalid literal for int()";
            size_t k = 0;
            while (m2[k] != '\0' && k + 1 < err_cap) { err[k] = m2[k]; ++k; }
            err[k] = '\0';
            return 0;
        }
        const char* m2 = "cannot convert to int";
        size_t k = 0;
        while (m2[k] != '\0' && k + 1 < err_cap) { err[k] = m2[k]; ++k; }
        err[k] = '\0';
        return 0;
    }

    if (n > 6 && t[0]=='f' && t[1]=='l' && t[2]=='o' && t[3]=='a' && t[4]=='t' && t[5]=='(' && t[n-1]==')') {
        char arg[512];
        size_t m = n - 7;
        if (m > sizeof(arg)-1) m = sizeof(arg)-1;
        for (size_t i = 0; i < m; ++i) arg[i] = t[6 + i];
        arg[m] = '\0';
        PyValue av;
        if (!eval_expr(p, arg, &av, err, err_cap)) return 0;
        if (av.typ == V_FLOAT) { *out = av; return 1; }
        if (av.typ == V_INT) { out->typ = V_FLOAT; out->f = static_cast<double>(av.i); return 1; }
        if (av.typ == V_STRING) {
            double x;
            if (parse_float64(av.s, &x)) { out->typ = V_FLOAT; out->f = x; return 1; }
            const char* m2 = "invalid literal for float()";
            size_t k = 0;
            while (m2[k] != '\0' && k + 1 < err_cap) { err[k] = m2[k]; ++k; }
            err[k] = '\0';
            return 0;
        }
        const char* m2 = "cannot convert to float";
        size_t k = 0;
        while (m2[k] != '\0' && k + 1 < err_cap) { err[k] = m2[k]; ++k; }
        err[k] = '\0';
        return 0;
    }

    // arithmetic (left-to-right split by last op)
    const char ops[4] = {'+', '-', '*', '/'};
    for (size_t oi = 0; oi < 4; ++oi) {
        char op = ops[oi];
        for (size_t i = n; i > 1; --i) {
            if (t[i - 1] == op) {
                char l[256];
                char r[256];
                size_t ln = i - 1;
                size_t rn = n - i;
                if (ln > 255) ln = 255;
                if (rn > 255) rn = 255;
                for (size_t k = 0; k < ln; ++k) l[k] = t[k];
                l[ln] = '\0';
                for (size_t k = 0; k < rn; ++k) r[k] = t[i + k];
                r[rn] = '\0';

                PyValue a, b;
                if (!eval_expr(p, l, &a, err, err_cap)) return 0;
                if (!eval_expr(p, r, &b, err, err_cap)) return 0;
                int ar = apply_arith(&a, &b, op, out, err, err_cap);
                if (ar == -1) return 0;
                if (ar == 1) return 1;
                const char* m2 = "unsupported operand types";
                size_t k = 0;
                while (m2[k] != '\0' && k + 1 < err_cap) { err[k] = m2[k]; ++k; }
                err[k] = '\0';
                return 0;
            }
        }
    }

    PyVar* var = find_var(p, t);
    if (var != nullptr) {
        *out = var->v;
        return 1;
    }

    const char* prefix = "name '";
    const char* suffix = "' is not defined";
    size_t pfx = c_len(prefix);
    size_t sfx = c_len(suffix);
    size_t nn = c_len(t);
    size_t pos = 0;
    for (size_t i = 0; i < pfx && pos + 1 < err_cap; ++i) err[pos++] = prefix[i];
    for (size_t i = 0; i < nn && pos + 1 < err_cap; ++i) err[pos++] = t[i];
    for (size_t i = 0; i < sfx && pos + 1 < err_cap; ++i) err[pos++] = suffix[i];
    err[pos] = '\0';
    return 0;
}

extern "C" uint32_t cpp_python_new() {
    for (uint32_t i = 0; i < 16; ++i) {
        if (!g_interps[i].used) {
            g_interps[i].used = 1;
            for (size_t k = 0; k < 128; ++k) {
                g_interps[i].vars[k].used = 0;
            }
            return i + 1;
        }
    }
    return 0;
}

extern "C" void cpp_python_free(uint32_t id) {
    PyInterp* p = get_interp(id);
    if (p == nullptr) return;
    p->used = 0;
}

extern "C" size_t cpp_python_eval(uint32_t id, const uint8_t* code, size_t code_len, uint8_t* out, size_t out_cap) {
    PyInterp* p = get_interp(id);
    if (p == nullptr) {
        if (out_cap >= 2) {
            out[0] = '1';
            out[1] = 'I';
        }
        return 2;
    }

    char code_buf[512];
    size_t n = code_len;
    if (n > sizeof(code_buf) - 1) n = sizeof(code_buf) - 1;
    for (size_t i = 0; i < n; ++i) code_buf[i] = static_cast<char>(code[i]);
    code_buf[n] = '\0';

    // Security gate.
    const char* blocked[] = {"import", "__", "eval", "exec", "open", "file", "compile"};
    for (size_t b = 0; b < sizeof(blocked) / sizeof(blocked[0]); ++b) {
        const char* k = blocked[b];
        size_t kl = c_len(k);
        for (size_t i = 0; i + kl <= n; ++i) {
            size_t j = 0;
            while (j < kl && code_buf[i + j] == k[j]) ++j;
            if (j == kl) {
                const char* m = "Forbidden operation";
                size_t ml = c_len(m);
                if (out_cap > 0) out[0] = '1';
                size_t wrote = 1;
                for (size_t q = 0; q < ml; ++q) {
                    if (wrote < out_cap) out[wrote] = static_cast<uint8_t>(m[q]);
                    ++wrote;
                }
                return wrote;
            }
        }
    }

    char t[512];
    copy_trimmed(code_buf, n, t, sizeof(t));
    size_t tn = c_len(t);

    PyValue result;
    char err[256];
    err[0] = '\0';

    // print(expr)
    if (tn > 7 && t[0]=='p' && t[1]=='r' && t[2]=='i' && t[3]=='n' && t[4]=='t' && t[5]=='(' && t[tn-1]==')') {
        char inner[512];
        size_t m = tn - 7;
        if (m > sizeof(inner) - 1) m = sizeof(inner) - 1;
        for (size_t i = 0; i < m; ++i) inner[i] = t[6 + i];
        inner[m] = '\0';
        if (!eval_expr(p, inner, &result, err, sizeof(err))) {
            if (out_cap > 0) out[0] = '1';
            size_t wrote = 1;
            for (size_t i = 0; err[i] != '\0'; ++i) {
                if (wrote < out_cap) out[wrote] = static_cast<uint8_t>(err[i]);
                ++wrote;
            }
            return wrote;
        }
        char s[256];
        value_to_str(&result, s, sizeof(s));
        if (out_cap > 0) out[0] = '0';
        size_t wrote = 1;
        for (size_t i = 0; s[i] != '\0'; ++i) {
            if (wrote < out_cap) out[wrote] = static_cast<uint8_t>(s[i]);
            ++wrote;
        }
        return wrote;
    }

    // assignment
    for (size_t i = 0; i < tn; ++i) {
        if (t[i] == '=') {
            if (i > 0 && t[i - 1] == '=') break;
            if (i + 1 < tn && t[i + 1] == '=') break;

            char lhs[64];
            char rhs[512];
            copy_trimmed(t, i, lhs, sizeof(lhs));
            copy_trimmed(t + i + 1, tn - i - 1, rhs, sizeof(rhs));

            if (!eval_expr(p, rhs, &result, err, sizeof(err))) {
                if (out_cap > 0) out[0] = '1';
                size_t wrote = 1;
                for (size_t q = 0; err[q] != '\0'; ++q) {
                    if (wrote < out_cap) out[wrote] = static_cast<uint8_t>(err[q]);
                    ++wrote;
                }
                return wrote;
            }

            PyVar* v = ensure_var(p, lhs);
            if (v == nullptr) {
                const char* m = "variable table full";
                if (out_cap > 0) out[0] = '1';
                size_t wrote = 1;
                for (size_t q = 0; m[q] != '\0'; ++q) {
                    if (wrote < out_cap) out[wrote] = static_cast<uint8_t>(m[q]);
                    ++wrote;
                }
                return wrote;
            }
            v->v = result;
            if (out_cap > 0) out[0] = '0';
            return 1;
        }
    }

    if (!eval_expr(p, t, &result, err, sizeof(err))) {
        if (out_cap > 0) out[0] = '1';
        size_t wrote = 1;
        for (size_t i = 0; err[i] != '\0'; ++i) {
            if (wrote < out_cap) out[wrote] = static_cast<uint8_t>(err[i]);
            ++wrote;
        }
        return wrote;
    }

    char s[256];
    value_to_str(&result, s, sizeof(s));
    if (out_cap > 0) out[0] = '0';
    size_t wrote = 1;
    for (size_t i = 0; s[i] != '\0'; ++i) {
        if (wrote < out_cap) out[wrote] = static_cast<uint8_t>(s[i]);
        ++wrote;
    }
    return wrote;
}
