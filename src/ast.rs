// MIT License
//
// Copyright (c) 2019 Alasdair Armstrong
//
// Permission is hereby granted, free of charge, to any person
// obtaining a copy of this software and associated documentation
// files (the "Software"), to deal in the Software without
// restriction, including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense, and/or sell copies
// of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
// BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
// ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::collections::{HashMap, HashSet};

use crate::concrete::Sbits;

#[derive(Clone, Debug)]
pub enum Ty<A> {
    Lint,
    Fint(u32),
    Constant(i128),
    Lbits,
    Sbits(u32),
    Fbits(u32),
    Unit,
    Bool,
    Bit,
    String,
    Real,
    Enum(A),
    Struct(A),
    Union(A),
    Vector(Box<Ty<A>>),
    List(Box<Ty<A>>),
    Ref(Box<Ty<A>>),
}

#[derive(Clone)]
pub enum Loc<A> {
    Id(A),
    Field(Box<Loc<A>>, A),
    Addr(Box<Loc<A>>),
}

#[derive(Clone, Copy)]
pub enum Op {
    Not,
    Or,
    And,
    Eq,
    Neq,
    Lteq,
    Lt,
    Slice(u32),
    Signed(u32),
    Unsigned(u32),
    Bvnot,
    Bvor,
    Bvxor,
    Bvand,
    Bvadd,
    Bvsub,
    Bvaccess,
    Concat,
    BitToBool,
}

#[derive(Clone, Copy)]
pub enum Bit {
    B0,
    B1,
}

#[derive(Clone)]
pub enum Exp<A> {
    Id(A),
    Ref(A),
    Bool(bool),
    Bit(Bit),
    Bits(Sbits),
    String(String),
    Unit,
    Int(i128),
    Struct(A, Vec<(A, Exp<A>)>),
    Kind(A, Box<Exp<A>>),
    Unwrap(A, Box<Exp<A>>),
    Field(Box<Exp<A>>, A),
    Call(Op, Vec<Exp<A>>),
}

#[derive(Clone)]
pub enum Instr<A> {
    Decl(A, Ty<A>),
    Init(A, Ty<A>, Exp<A>),
    Jump(Exp<A>, usize),
    Goto(usize),
    Copy(Loc<A>, Exp<A>),
    Call(Loc<A>, bool, A, Vec<Exp<A>>),
    Primop(Loc<A>, A, Vec<Exp<A>>),
    Failure,
    Arbitrary,
    End,
}

#[derive(Clone)]
pub enum Def<A> {
    Register(A, Ty<A>),
    Let(Vec<(A, Ty<A>)>, Vec<Instr<A>>),
    Enum(A, Vec<A>),
    Struct(A, Vec<(A, Ty<A>)>),
    Union(A, Vec<(A, Ty<A>)>),
    Val(A, Vec<Ty<A>>, Ty<A>),
    Fn(A, Vec<A>, Vec<Instr<A>>),
}

pub struct Symtab<'ast> {
    symbols: Vec<&'ast str>,
    table: HashMap<&'ast str, u32>,
    next: u32,
}

pub static RETURN: u32 = 0;

impl<'ast> Symtab<'ast> {
    pub fn intern(&mut self, sym: &'ast str) -> u32 {
        match self.table.get(sym) {
            None => {
                let n = self.next;
                self.symbols.push(sym);
                self.table.insert(sym, n);
                self.next += 1;
                n
            }
            Some(n) => *n,
        }
    }

    pub fn to_str(&self, n: u32) -> &'ast str {
        self.symbols[n as usize]
    }

    pub fn new() -> Self {
        let mut symtab = Symtab { symbols: Vec::new(), table: HashMap::new(), next: 0 };
        symtab.intern("return");
        symtab.intern("current_exception");
        symtab.intern("have_exception");
        symtab.intern("zsail_assert");
        symtab.intern("zinternal_vector_init");
        symtab.intern("zinternal_vector_update");
        symtab
    }

    pub fn lookup(&self, sym: &str) -> u32 {
        *self.table.get(sym).expect(&format!("Could not find symbol: {}", sym))
    }

    pub fn intern_ty(&mut self, ty: &'ast Ty<String>) -> Ty<u32> {
        use Ty::*;
        match ty {
            Lint => Lint,
            Fint(sz) => Fint(*sz),
            Constant(c) => Constant(*c),
            Lbits => Lbits,
            Sbits(sz) => Sbits(*sz),
            Fbits(sz) => Fbits(*sz),
            Unit => Unit,
            Bool => Bool,
            Bit => Bit,
            String => String,
            Real => Real,
            Enum(e) => Enum(self.lookup(e)),
            Struct(s) => Struct(self.lookup(s)),
            Union(u) => Union(self.lookup(u)),
            Vector(ty) => Vector(Box::new(self.intern_ty(ty))),
            List(ty) => List(Box::new(self.intern_ty(ty))),
            Ref(ty) => Ref(Box::new(self.intern_ty(ty))),
        }
    }

    pub fn intern_loc(&mut self, loc: &'ast Loc<String>) -> Loc<u32> {
        use Loc::*;
        match loc {
            Id(v) => Id(self.lookup(v)),
            Field(loc, field) => Field(Box::new(self.intern_loc(loc)), self.lookup(field)),
            Addr(loc) => Addr(Box::new(self.intern_loc(loc))),
        }
    }

    pub fn intern_exp(&mut self, exp: &'ast Exp<String>) -> Exp<u32> {
        use Exp::*;
        match exp {
            Id(v) => Id(self.lookup(v)),
            Ref(reg) => Ref(self.lookup(reg)),
            Bool(b) => Bool(*b),
            Bit(b) => Bit(*b),
            Bits(bv) => Bits(*bv),
            String(s) => String(s.clone()),
            Unit => Unit,
            Int(i) => Int(*i),
            Struct(s, fields) => Struct(
                self.lookup(s),
                fields.iter().map(|(field, exp)| (self.lookup(field), self.intern_exp(exp))).collect(),
            ),
            Kind(ctor, exp) => Kind(self.lookup(ctor), Box::new(self.intern_exp(exp))),
            Unwrap(ctor, exp) => Kind(self.lookup(ctor), Box::new(self.intern_exp(exp))),
            Field(exp, field) => Field(Box::new(self.intern_exp(exp)), self.lookup(field)),
            Call(op, args) => Call(*op, args.iter().map(|exp| self.intern_exp(exp)).collect()),
        }
    }

    pub fn intern_instr(&mut self, instr: &'ast Instr<String>) -> Instr<u32> {
        use Instr::*;
        match instr {
            Decl(v, ty) => Decl(self.intern(v), self.intern_ty(ty)),
            Init(v, ty, exp) => {
                let exp = self.intern_exp(exp);
                Init(self.intern(v), self.intern_ty(ty), exp)
            }
            Jump(exp, target) => Jump(self.intern_exp(exp), *target),
            Goto(target) => Goto(*target),
            Copy(loc, exp) => Copy(self.intern_loc(loc), self.intern_exp(exp)),
            Call(loc, ext, f, args) => {
                let loc = self.intern_loc(loc);
                let args = args.iter().map(|exp| self.intern_exp(exp)).collect();
                Call(loc, *ext, self.lookup(f), args)
            }
            Failure => Failure,
            Arbitrary => Arbitrary,
            End => End,
            // We split calls into primops/regular calls later, so
            // this shouldn't exist yet.
            Primop(loc, f, args) => unreachable!("Primop in intern_instr"),
        }
    }

    pub fn intern_def(&mut self, def: &'ast Def<String>) -> Def<u32> {
        use Def::*;
        match def {
            Register(reg, ty) => Register(self.intern(reg), self.intern_ty(ty)),
            Let(bindings, setup) => {
                let bindings = bindings.iter().map(|(v, ty)| (self.intern(v), self.intern_ty(ty))).collect();
                let setup = setup.iter().map(|instr| self.intern_instr(instr)).collect();
                Let(bindings, setup)
            }
            Enum(e, ctors) => Enum(self.intern(e), ctors.iter().map(|ctor| self.intern(ctor)).collect()),
            Struct(s, fields) => {
                let fields = fields.iter().map(|(field, ty)| (self.intern(field), self.intern_ty(ty))).collect();
                Struct(self.intern(s), fields)
            }
            Union(u, ctors) => {
                let ctors = ctors.iter().map(|(ctor, ty)| (self.intern(ctor), self.intern_ty(ty))).collect();
                Struct(self.intern(u), ctors)
            }
            Val(f, args, ret) => {
                Val(self.intern(f), args.iter().map(|ty| self.intern_ty(ty)).collect(), self.intern_ty(ret))
            }
            Fn(f, args, body) => {
                let args = args.iter().map(|arg| self.intern(arg)).collect();
                let body = body.iter().map(|instr| self.intern_instr(instr)).collect();
                Fn(self.lookup(f), args, body)
            }
        }
    }

    pub fn intern_defs(&mut self, defs: &'ast Vec<Def<String>>) -> Vec<Def<u32>> {
        defs.iter().map(|def| self.intern_def(def)).collect()
    }
}

type Fn<'ast> = (Vec<(u32, Ty<u32>)>, Ty<u32>, &'ast [Instr<u32>]);

pub struct SharedState<'ast> {
    pub functions: HashMap<u32, Fn<'ast>>,
    pub symtab: Symtab<'ast>,
    pub primops: HashSet<u32>,
}

impl<'ast> SharedState<'ast> {
    pub fn new(symtab: Symtab<'ast>, defs: &'ast [Def<u32>], primops: HashSet<u32>) -> Self {
        let mut vals = HashMap::new();
        let mut functions: HashMap<u32, Fn<'ast>> = HashMap::new();
        for def in defs {
            match def {
                Def::Val(f, arg_tys, ret_ty) => {
                    vals.insert(f, (arg_tys, ret_ty));
                }
                Def::Fn(f, args, body) => match vals.get(f) {
                    None => panic!("Found fn without a val when creating the global state!"),
                    Some((arg_tys, ret_ty)) => {
                        assert!(arg_tys.len() == args.len());
                        let args = args.iter().zip(arg_tys.iter()).map(|(id, arg)| (*id, arg.clone())).collect();
                        functions.insert(*f, (args, (*ret_ty).clone(), body));
                    }
                },
                _ => (),
            }
        }
        SharedState { functions, symtab, primops }
    }
}

/// Change Calls without implementations into Primops
pub fn insert_primops(defs: &mut [Def<u32>]) -> HashSet<u32> {
    let mut primops: HashSet<u32> = HashSet::new();
    for def in defs.iter() {
        match def {
            Def::Val(f, _, _) => {
                primops.insert(*f);
            }
            Def::Fn(f, _, _) => {
                primops.remove(f);
            }
            _ => (),
        }
    }
    for def in defs.iter_mut() {
        match def {
            Def::Fn(f, args, body) => {
                *def = Def::Fn(
                    *f,
                    args.to_vec(),
                    body.to_vec()
                        .into_iter()
                        .map(|instr| match &instr {
                            Instr::Call(loc, _, f, args) if primops.contains(&f) => {
                                Instr::Primop(loc.clone(), *f, args.to_vec())
                            }
                            _ => instr,
                        })
                        .collect(),
                )
            }
            _ => (),
        }
    }
    primops
}
