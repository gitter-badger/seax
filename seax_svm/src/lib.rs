#![crate_name = "seax_svm"]
#![crate_type = "lib"]
#![feature(box_syntax,box_patterns,core)]

/// Contains the Seax Virtual Machine (SVM) and miscellaneous
/// support code.
pub mod svm {
    pub use self::slist::List;
    pub use self::slist::List::{Cons,Nil};
    pub use self::slist::Stack;
    pub use self::cell::{SVMCell,Atom,Inst};
    use self::cell::SVMCell::*;
    use self::cell::Atom::*;
    use self::cell::Inst::*;

    /// Singly-linked list and stack implementations.
    ///
    /// `List<T>` is a singly-linked cons list with boxed items. `Stack<T>` is
    ///  defined as a trait providing stack operations(`push()`, `pop()`, and
    ///  `peek()`), and an implementation for `List`.
    #[macro_use]
    pub mod slist;

    /// SVM cell types.
    ///
    /// A cell in the VM can be either an atom (single item, either unsigned
    /// int, signed int, float, or string), a pointer to a list cell, or an
    /// instruction.
    pub mod cell;

    #[cfg(test)]
    mod tests;

    /// Represents a SVM machine state
    #[derive(PartialEq,Clone,Debug)]
    pub struct State {
        stack:  List<SVMCell>,
        env:  List<SVMCell>,
        control:  List<SVMCell>,
        dump:  List<SVMCell>
    }

    impl State {

        /// Creates a new empty state
        pub fn new() -> State {
            State {
                stack: Stack::empty(),
                env: Stack::empty(),
                control: Stack::empty(),
                dump: Stack::empty()
            }
        }

        /// Evaluates an instruction.
        ///
        /// Evaluates an instruction against a state, returning a new state.
        /// TODO: rewrite me to use the next instruction on the control stack,
        /// rather than a parameter.
        pub fn eval(self) -> State {
            match self.control.pop() {
                // NIL: pop an empty list onto the stack
                Some((InstCell(NIL), new_control @ _)) => {
                    State {
                        stack: self.stack.push(ListCell(box List::new())),
                        env: self.env,
                        control: new_control,
                        dump: self.dump
                    }
                }
                // LDC: load constant
                Some((InstCell(LDC), new_control @ _)) => {
                    let (atom,newer_control) = new_control.pop().unwrap();
                    State {
                        stack: self.stack.push(atom),
                        env: self.env,
                        control: newer_control,
                        dump: self.dump
                    }
                },
                // LD: load variable
                Some((InstCell(LD), new_control @ _)) => {
                    match new_control.pop() {
                       Some((ListCell(
                            box Cons(AtomCell(SInt(level)),
                            box Cons(AtomCell(SInt(pos)),
                            box Nil))
                        ), newer_control @ _)) => {
                            let environment = match self.env[level] {
                                SVMCell::ListCell(ref l) => l.clone(),
                                _ => panic!("[LD]: Fatal: expected list in $e, found {:?}",self.env[level])
                            };
                            State {
                                stack: self.stack.push(environment[pos].clone()),
                                env: self.env,
                                control: newer_control,
                                dump: self.dump
                            }
                        },
                       it @ _ => panic!("[LD] Fatal: expected pair, found {:?}", it)
                    }
                },

                // LDF: load function
                Some((InstCell(LDF), new_control @ _)) => {
                    let (func, newer_control) = new_control.pop().unwrap();
                    State {
                        stack: self.stack.push(ListCell(box list!(func,self.env[0usize].clone()))),
                        env: self.env,
                        control: newer_control,
                        dump: self.dump
                    }
                },

                Some((InstCell(JOIN), new_control @ _)) => {
                    let (top, new_dump) = self.dump.pop().unwrap();
                    State {
                        stack: self.stack,
                        env: self.env,
                        control: new_control.push(top),
                        dump: new_dump
                    }
                },
                Some((InstCell(ADD), new_control @ _)) => {
                    let (op1, new_stack) = self.stack.pop().unwrap();
                    match op1 {
                        AtomCell(a) => {
                            let (op2, newer_stack) = new_stack.pop().unwrap();
                            match op2 {
                                AtomCell(b) => State {
                                    stack: newer_stack.push(AtomCell(a + b)),
                                    env: self.env,
                                    control: new_control,
                                    dump: self.dump
                                },
                                b => panic!("[ADD] TypeError: expected compatible operands, found (ADD {:?} {:?})", a, b)
                            }
                        },
                        _ => panic!("[ADD]: Expected first operand to be atom, found list or instruction"),
                    }
                },
                Some((InstCell(SUB), new_control @ _)) => {
                    let (op1, new_stack) = self.stack.pop().unwrap();
                    match op1 {
                        AtomCell(a) => {
                            let (op2, newer_stack) = new_stack.pop().unwrap();
                            match op2 {
                                AtomCell(b) => State {
                                    stack: newer_stack.push(AtomCell(a - b)),
                                    env: self.env,
                                    control: new_control,
                                    dump: self.dump
                                },
                                b => panic!("[SUB] TypeError: expected compatible operands, found (SUB {:?} {:?})", a, b)
                            }
                        },
                        _ => panic!("[SUB]: Expected first operand to be atom, found list or instruction"),
                    }
                },
                Some((InstCell(FDIV), new_control @ _)) => {
                    let (op1, new_stack) = self.stack.pop().unwrap();
                    match op1 {
                        AtomCell(a) => {
                            let (op2, newer_stack) = new_stack.pop().unwrap();
                            match op2 {
                                AtomCell(b) => State {
                                    stack: newer_stack.push(AtomCell(
                                        match (a, b) {
                                            // same type: coerce to float
                                            (SInt(a), SInt(b))      => Float(a as f64 / b as f64),
                                            (UInt(a), UInt(b))      => Float(a as f64 / b as f64),
                                            (Float(a), Float(b))    => Float(a / b),
                                            // float + int: coerce to float
                                            (Float(a), SInt(b))     => Float(a / b as f64),
                                            (Float(a), UInt(b))     => Float(a / b as f64),
                                            (SInt(a), Float(b))     => Float(a as f64 / b),
                                            (UInt(a), Float(b))     => Float(a as f64 / b),
                                            // uint + sint: coerce to float
                                            (UInt(a), SInt(b))      => Float(a as f64 / b as f64),
                                            (SInt(a), UInt(b))      => Float(a as f64 / b as f64),
                                            // char + any: coerce to int -> float
                                            // but if you ever actually do this, then ...wat?
                                            (Char(a), Char(b))      => Float(a as u8 as f64 / b as u8 as f64),
                                            (Char(a), UInt(b))      => Float(a as u8 as f64 / b as f64),
                                            (Char(a), SInt(b))      => Float(a as u8 as f64 / b as f64),
                                            (Char(a), Float(b))     => Float(a as u8 as f64 / b as f64),
                                            (UInt(a), Char(b))      => Float(a as f64 / b as u8 as f64),
                                            (SInt(a), Char(b))      => Float(a as f64 / b as u8 as f64),
                                            (Float(a), Char(b))     => Float(a as f64 / b as u8 as f64)
                                        }
                                        )),
                                    env: self.env,
                                    control: new_control,
                                    dump: self.dump
                                },
                                b => panic!("[FDIV] TypeError: expected compatible operands, found (FDIV {:?} {:?})", a, b)
                            }
                        },
                        _ => panic!("[FDIV]: Expected first operand to be atom, found list or instruction"),
                    }
                },
                Some((InstCell(DIV), new_control @ _)) => {
                    let (op1, new_stack) = self.stack.pop().unwrap();
                    match op1 {
                        AtomCell(a) => {
                            let (op2, newer_stack) = new_stack.pop().unwrap();
                            match op2 {
                                AtomCell(b) => State {
                                    stack: newer_stack.push(AtomCell(a / b)),
                                    env: self.env,
                                    control: new_control,
                                    dump: self.dump
                                },
                                b => panic!("[DIV] TypeError: expected compatible operands, found (DIV {:?} {:?})", a, b)
                            }
                        },
                        _ => panic!("[DIV]: Expected first operand to be atom, found list or instruction"),
                    }
                },
                Some((InstCell(MUL), new_control @ _)) => {
                    let (op1, new_stack) = self.stack.pop().unwrap();
                    match op1 {
                        AtomCell(a) => {
                            let (op2, newer_stack) = new_stack.pop().unwrap();
                            match op2 {
                                AtomCell(b) => State {
                                    stack: newer_stack.push(AtomCell(a * b)),
                                    env: self.env,
                                    control: new_control,
                                    dump: self.dump
                                },
                                b => panic!("[MUL] TypeError: expected compatible operands, found (MUL {:?} {:?})", a, b)
                            }
                        },
                        _ => panic!("[MUL]: Expected first operand to be atom, found list or instruction"),
                    }
                },
                Some((InstCell(MOD), new_control @ _)) => {
                    let (op1, new_stack) = self.stack.pop().unwrap();
                    match op1 {
                        AtomCell(a) => {
                            let (op2, newer_stack) = new_stack.pop().unwrap();
                            match op2 {
                                AtomCell(b) => State {
                                    stack: newer_stack.push(AtomCell(a % b)),
                                    env: self.env,
                                    control: new_control,
                                    dump: self.dump
                                },
                                b => panic!("[MOD] TypeError: expected compatible operands, found (MOD {:?} {:?})", a, b)
                            }
                        },
                        _ => panic!("[MOD]: Expected first operand to be atom, found list or instruction"),
                    }
                },
                Some((InstCell(EQ), new_control @ _)) => {
                    let (op1, new_stack) = self.stack.pop().unwrap();
                    let (op2, newer_stack) = new_stack.pop().unwrap();
                    match (op1,op2) {
                        (AtomCell(a), AtomCell(b)) => State {
                            stack: newer_stack.push(
                                match a == b {
                                    true    => ListCell(box list!(AtomCell(SInt(1)))),
                                    false   => ListCell(box Nil)
                                }
                            ),
                            env: self.env,
                            control: new_control,
                            dump: self.dump
                        },
                    (_,_) => unimplemented!()
                    }
                },
                Some((InstCell(GT), new_control @ _)) => {
                    let (op1, new_stack) = self.stack.pop().unwrap();
                    let (op2, newer_stack) = new_stack.pop().unwrap();
                    match (op1,op2) {
                        (AtomCell(a), AtomCell(b)) => State {
                            stack: newer_stack.push(
                                match a > b {
                                    true    => ListCell(box list!(AtomCell(SInt(1)))),
                                    false   => ListCell(box Nil)
                                }
                            ),
                            env: self.env,
                            control: new_control,
                            dump: self.dump
                        },
                    (_,_) => unimplemented!()
                    }
                },
                Some((InstCell(GTE), new_control @ _)) => {
                    let (op1, new_stack) = self.stack.pop().unwrap();
                    let (op2, newer_stack) = new_stack.pop().unwrap();
                    match (op1,op2) {
                        (AtomCell(a), AtomCell(b)) => State {                            stack: newer_stack.push(
                                match a >= b {
                                    true    => ListCell(box list!(AtomCell(SInt(1)))),
                                    false   => ListCell(box Nil)
                                }
                            ),
                            env: self.env,
                            control: new_control,
                            dump: self.dump
                        },
                    (_,_) => unimplemented!()
                    }
                },
                Some((InstCell(LT), new_control @ _)) => {
                    let (op1, new_stack) = self.stack.pop().unwrap();
                    let (op2, newer_stack) = new_stack.pop().unwrap();
                    match (op1,op2) {
                        (AtomCell(a), AtomCell(b)) => State {
                            stack: newer_stack.push(
                                match a < b {
                                    true    => ListCell(box list!(AtomCell(SInt(1)))),
                                    false   => ListCell(box Nil)
                                }
                            ),
                            env: self.env,
                            control: new_control,
                            dump: self.dump
                        },
                    (_,_) => unimplemented!()
                    }
                },
                Some((InstCell(LTE), new_control @ _)) => {
                    let (op1, new_stack) = self.stack.pop().unwrap();
                    let (op2, newer_stack) = new_stack.pop().unwrap();
                    match (op1,op2) {
                        (AtomCell(a), AtomCell(b)) => State {
                            stack: newer_stack.push(
                                match a <= b {
                                    true    => ListCell(box list!(AtomCell(SInt(1)))),
                                    false   => ListCell(box Nil)
                                }
                            ),
                            env: self.env,
                            control: new_control,
                            dump: self.dump
                        },
                    (_,_) => unimplemented!()
                    }
                },
                Some((InstCell(ATOM), new_control @ _)) => {
                    let (target, new_stack) = self.stack.pop().unwrap();
                    State {
                        stack: new_stack.push(
                            match target {
                                AtomCell(_) => ListCell(box list!(AtomCell(SInt(1)))),
                                _           => ListCell(box Nil)
                            }
                            ),
                        env: self.env,
                        control: new_control,
                        dump: self.dump
                    }
                },
                Some((InstCell(AP), new_control @ _)) => {
                    match self.stack.pop().unwrap() {
                        (ListCell(box Cons(ListCell(box func), box Cons(ListCell(box params), box Nil))), new_stack) => State {
                            stack: new_stack,
                            env: params,
                            control: func,
                            dump: self.dump.push(ListCell(box self.env)).push(ListCell(box new_control))
                        },
                        (_, thing) => panic!("[AP]: Fatal: Expected closure on stack, got:\n{:?}", thing)
                    }
                },
                Some((InstCell(RAP), new_control @ _)) => {
                     match self.stack.pop().unwrap() {
                        (ListCell(box Cons(ListCell(box func), box Cons(ListCell(box params), box Nil))), new_stack) => {
                            match new_stack.pop() {
                                Some((v @ ListCell(_), newer_stack)) => {
                                    State {
                                        stack: Stack::empty(),
                                        env: params.push(v),
                                        control: func,
                                        dump: self.dump
                                                .push(ListCell(box new_control))
                                                .push(ListCell(box self.env.pop().unwrap().1))
                                                .push(ListCell(box newer_stack))
                                    }
                                },
                                Some((thing, _)) => panic!("[AP]: Fatal: Expected closure on stack, got:\n{:?}", thing),
                                None => panic!("[AP]: Fatal: expected non-empty stack")
                            }
                        },
                        (_, thing) => panic!("[AP]: Fatal: Expected closure on stack, got:\n{:?}", thing)
                    }
                },
                Some((InstCell(RET), _)) => {
                    let (head, _) = self.stack.pop().unwrap();
                    let (new_stack, new_dump) = {
                        match self.dump.pop().unwrap()  {
                            (ListCell(s), d @ _)    => (*s, d),
                            it @ (AtomCell(_),_)    => (list!(it.0), it.1),
                            _                       => panic!("[RET]: Expected non-empty stack")
                        }
                    };
                    let (new_env, newer_dump) = {
                        match new_dump.pop().unwrap() {
                            (ListCell(e), d @ _)    => (*e, d),
                            _                       => panic!("[RET]: Expected new environment on dump stack")
                        }
                    };
                    let (newer_control, newest_dump) = {
                        match newer_dump.pop().unwrap()  {
                            (ListCell(c), d @ _)    => (*c, d),
                            it @ (InstCell(_),_)    => (list!(it.0), it.1),
                            _                       => panic!("[RET]: Expected new control stack on dump stack")
                        }
                    };
                    State {
                        stack: new_stack.push(head),
                        env: new_env,
                        control: newer_control,
                        dump: newest_dump
                    }
                },
                Some((InstCell(DUM), new_control @ _)) => {
                    State {
                        stack: self.stack,
                        env: self.env.push(ListCell(list!())),
                        control: new_control,
                        dump: self.dump
                    }
                },
                Some((InstCell(SEL), new_control @ _)) => {
                    match new_control.pop() {
                        Some((ListCell(box true_case), newer_control)) => {
                            match newer_control.pop() {
                                Some((ListCell(box false_case), newest_control)) => {
                                    match self.stack.pop() {
                                        Some((ListCell(box Nil), new_stack)) => { // false case
                                            State {
                                                stack: new_stack,
                                                env: self.env,
                                                control: false_case,
                                                dump: self.dump.push(ListCell(box newest_control))
                                            }
                                        },
                                        Some((_, new_stack)) => { // true case
                                            State {
                                                stack: new_stack,
                                                env: self.env,
                                                control: true_case,
                                                dump: self.dump.push(ListCell(box newest_control))
                                            }
                                        },
                                        None => panic!("[SEL]: expected non-empty stack")
                                    }
                                },
                                Some((thing, _)) => panic!("[SEL]: expected list on control, found {:?}", thing),
                                None             => panic!("[SEL]: expected list on control, found nothing")
                            }
                        },
                        Some((thing, _)) => panic!("[SEL]: expected list on control, found {:?}", thing),
                        None             => panic!("[SEL]: expected list on control, found nothing")

                    }
                },
                Some((InstCell(CAR), new_control @ _)) => {
                    match self.stack.pop() {
                        Some((ListCell(box Cons(car, _)), new_stack)) => State {
                            stack: new_stack.push(car),
                            env: self.env,
                            control: new_control,
                            dump: self.dump
                        },
                        Some((ListCell(box Nil), _)) => panic!("[CAR]: expected non-empty list, found Nil"),
                        Some((thing, _))             => panic!("[CAR]: expected non-empty list, found {:?}", thing),
                        None                         => panic!("[CAR]: Expected non-empty list, found nothing")
                    }
                },
                Some((InstCell(CDR), new_control @ _)) => {
                    match self.stack.pop() {
                        Some((ListCell(box Cons(_, cdr)), new_stack)) => State {
                            stack: new_stack.push(ListCell(cdr)),
                            env: self.env,
                            control: new_control,
                            dump: self.dump
                        },
                        Some((ListCell(box Nil), _)) => panic!("[CDR]: expected non-empty list, found Nil"),
                        Some((thing, _))             => panic!("[CDR]: expected non-empty list, found {:?}", thing),
                        None                         => panic!("[CDR]: Expected non-empty list, found nothing")
                    }
                },
                Some((InstCell(CONS), new_control @ _)) => {
                    match self.stack.pop() {
                        Some((thing, new_stack)) => {
                            match new_stack.pop() {
                                Some((ListCell(list), newer_stack)) => {
                                    State {
                                        stack: newer_stack.push(ListCell(box Cons(thing, list))),
                                        env: self.env,
                                        control: new_control,
                                        dump: self.dump
                                    }
                                },
                                Some((thing_else, _)) => panic!("[CONS]: Expected a list on the stack, found {:?}", thing_else),
                                None               => panic!("[CONS]: Expected a list on the stack, found nothing.")
                            }
                        },
                        None => panic!("[CONS]: Expected an item on the stack, found nothing")
                    }
                },
                None => {panic!("[eval]: expected an instruction on control stack")}
                it @ _ => { panic!("[eval]: Tried to evaluate an unsupported cell type {:?}.", it) }
            }
        }
    }


    /// Evaluates a program.
    ///
    /// Evaluates a program (control stack) and returns the final state.
    /// TODO: add (optional?) parameters for stdin and stdout
    pub fn eval_program(program: List<SVMCell>) -> List<SVMCell> {
        let mut machine = State {
            stack:      Stack::empty(),
            env:        Stack::empty(),
            control:    program,
            dump:       Stack::empty()
        };
        // while there are more instructions,
        while machine.control.peek() != None {  //TODO: this is kinda heavyweight
            machine = machine.eval() // continue evaling
        };
        machine.stack
    }

}
