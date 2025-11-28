use std::collections::HashMap;
use std::fmt;

pub struct PythonInterpreter {
    globals: HashMap<String, PythonValue>,
    output: Vec<String>,
}

impl Default for PythonInterpreter { fn default() -> Self { Self::new() } }

#[derive(Debug, Clone)]
pub enum PythonValue {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    None,
    List(Vec<PythonValue>),
}

impl fmt::Display for PythonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PythonValue::Int(i) => write!(f, "{}", i),
            PythonValue::Float(fl) => write!(f, "{}", fl),
            PythonValue::String(s) => write!(f, "{}", s),
            PythonValue::Bool(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            PythonValue::None => write!(f, "None"),
            PythonValue::List(items) => {
                let strs: Vec<String> = items.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", strs.join(", "))
            }
        }
    }
}

impl PythonInterpreter {
    pub fn new() -> Self {
        PythonInterpreter {
            globals: HashMap::new(),
            output: Vec::new(),
        }
    }

    pub fn eval(&mut self, code: &str) -> Result<String, String> {
        // Security: Block dangerous operations
        if code.contains("import")
            || code.contains("__")
            || code.contains("eval")
            || code.contains("exec")
            || code.contains("open")
            || code.contains("file")
            || code.contains("compile")
        {
            return Err("Forbidden operation".to_string());
        }

        let trimmed = code.trim();

        // Handle print() function
        if trimmed.starts_with("print(") && trimmed.ends_with(")") {
            let content = &trimmed[6..trimmed.len() - 1];
            let result = self.eval_expression(content)?;
            self.output.push(result.to_string());
            return Ok(result.to_string());
        }

        // Handle variable assignment
        if let Some(eq_pos) = trimmed.find('=') {
            if !trimmed[..eq_pos].contains('>')
                && !trimmed[..eq_pos].contains('<')
                && !trimmed[..eq_pos].contains('!')
                && !trimmed[..eq_pos].contains('=')
            {
                let var_name = trimmed[..eq_pos].trim().to_string();
                let expr = trimmed[eq_pos + 1..].trim();
                let value = self.eval_expression(expr)?;
                self.globals.insert(var_name, value);
                return Ok(String::new());
            }
        }

        // Handle expressions
        let result = self.eval_expression(trimmed)?;
        Ok(result.to_string())
    }

    fn eval_expression(&self, expr: &str) -> Result<PythonValue, String> {
        let expr = expr.trim();

        // String literals
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            return Ok(PythonValue::String(expr[1..expr.len() - 1].to_string()));
        }

        // Boolean literals
        if expr == "True" {
            return Ok(PythonValue::Bool(true));
        }
        if expr == "False" {
            return Ok(PythonValue::Bool(false));
        }
        if expr == "None" {
            return Ok(PythonValue::None);
        }

        // Number literals
        if let Ok(i) = expr.parse::<i64>() {
            return Ok(PythonValue::Int(i));
        }
        if let Ok(f) = expr.parse::<f64>() {
            return Ok(PythonValue::Float(f));
        }

        // Variable lookup
        if let Some(value) = self.globals.get(expr) {
            return Ok(value.clone());
        }

        // Built-in functions
        if expr.starts_with("len(") && expr.ends_with(")") {
            let arg = &expr[4..expr.len() - 1];
            let val = self.eval_expression(arg)?;
            match val {
                PythonValue::String(s) => Ok(PythonValue::Int(s.len() as i64)),
                PythonValue::List(l) => Ok(PythonValue::Int(l.len() as i64)),
                _ => Err("len() requires string or list".to_string()),
            }
        } else if expr.starts_with("str(") && expr.ends_with(")") {
            let arg = &expr[4..expr.len() - 1];
            let val = self.eval_expression(arg)?;
            Ok(PythonValue::String(val.to_string()))
        } else if expr.starts_with("int(") && expr.ends_with(")") {
            let arg = &expr[4..expr.len() - 1];
            let val = self.eval_expression(arg)?;
            match val {
                PythonValue::Int(i) => Ok(PythonValue::Int(i)),
                PythonValue::Float(f) => Ok(PythonValue::Int(f as i64)),
                PythonValue::String(s) => s
                    .parse::<i64>()
                    .map(PythonValue::Int)
                    .map_err(|_| "invalid literal for int()".to_string()),
                _ => Err("cannot convert to int".to_string()),
            }
        } else if expr.starts_with("float(") && expr.ends_with(")") {
            let arg = &expr[6..expr.len() - 1];
            let val = self.eval_expression(arg)?;
            match val {
                PythonValue::Float(f) => Ok(PythonValue::Float(f)),
                PythonValue::Int(i) => Ok(PythonValue::Float(i as f64)),
                PythonValue::String(s) => s
                    .parse::<f64>()
                    .map(PythonValue::Float)
                    .map_err(|_| "invalid literal for float()".to_string()),
                _ => Err("cannot convert to float".to_string()),
            }
        } else if expr.contains('+')
            || expr.contains('-')
            || expr.contains('*')
            || expr.contains('/')
        {
            self.eval_arithmetic(expr)
        } else {
            Err(format!("name '{}' is not defined", expr))
        }
    }

    fn eval_arithmetic(&self, expr: &str) -> Result<PythonValue, String> {
        // Simple arithmetic evaluation (left to right, no precedence)
        let ops = ['+', '-', '*', '/'];

        for op in ops.iter() {
            if let Some(pos) = expr.rfind(*op) {
                if pos > 0 && pos < expr.len() - 1 {
                    let left = self.eval_expression(&expr[..pos])?;
                    let right = self.eval_expression(&expr[pos + 1..])?;

                    return match (left, right) {
                        (PythonValue::Int(a), PythonValue::Int(b)) => match op {
                            '+' => Ok(PythonValue::Int(a + b)),
                            '-' => Ok(PythonValue::Int(a - b)),
                            '*' => Ok(PythonValue::Int(a * b)),
                            '/' => {
                                if b == 0 {
                                    Err("division by zero".to_string())
                                } else {
                                    Ok(PythonValue::Float(a as f64 / b as f64))
                                }
                            }
                            _ => Err("unsupported operation".to_string()),
                        },
                        (PythonValue::Float(a), PythonValue::Float(b)) => match op {
                            '+' => Ok(PythonValue::Float(a + b)),
                            '-' => Ok(PythonValue::Float(a - b)),
                            '*' => Ok(PythonValue::Float(a * b)),
                            '/' => {
                                if b == 0.0 {
                                    Err("division by zero".to_string())
                                } else {
                                    Ok(PythonValue::Float(a / b))
                                }
                            }
                            _ => Err("unsupported operation".to_string()),
                        },
                        (PythonValue::Float(a), PythonValue::Int(b)) => {
                            let b = b as f64;
                            match op {
                                '+' => Ok(PythonValue::Float(a + b)),
                                '-' => Ok(PythonValue::Float(a - b)),
                                '*' => Ok(PythonValue::Float(a * b)),
                                '/' => {
                                    if b == 0.0 {
                                        Err("division by zero".to_string())
                                    } else {
                                        Ok(PythonValue::Float(a / b))
                                    }
                                }
                                _ => Err("unsupported operation".to_string()),
                            }
                        }
                        (PythonValue::Int(a), PythonValue::Float(b)) => {
                            let a = a as f64;
                            match op {
                                '+' => Ok(PythonValue::Float(a + b)),
                                '-' => Ok(PythonValue::Float(a - b)),
                                '*' => Ok(PythonValue::Float(a * b)),
                                '/' => {
                                    if b == 0.0 {
                                        Err("division by zero".to_string())
                                    } else {
                                        Ok(PythonValue::Float(a / b))
                                    }
                                }
                                _ => Err("unsupported operation".to_string()),
                            }
                        }
                        (PythonValue::String(a), PythonValue::String(b)) if *op == '+' => {
                            Ok(PythonValue::String(format!("{}{}", a, b)))
                        }
                        _ => Err("unsupported operand types".to_string()),
                    };
                }
            }
        }

        Err("invalid expression".to_string())
    }
}
