//! Parses `.obj` format which stores 3D mesh data

use std::io::BufRead;
use std::collections::{HashMap, VecMap};
use std::simd::f32x4;
use error::ObjResult;
use raw::lexer::lex;

/// Parses a string into number.
macro_rules! n {
    ($input:expr) => ( try!($input.parse()) )
}

/// Parses &[&str] into &[f32].
macro_rules! f {
    ($args:expr) => (
        &{
            let mut ret = Vec::new();
            for &arg in $args.iter() {
                ret.push(try!(arg.parse::<f32>()))
            }
            ret
        }[..]
    )
}

/// Splits a string with '/'.
macro_rules! s {
    ($param:ident) => ( &$param.split('/').collect::<Vec<&str>>()[..] )
}

/// Parses a wavefront `.obj` format.
pub fn parse_obj<T: BufRead>(input: T) -> ObjResult<RawObj> {
    let mut name = None;
    let mut material_libraries = Vec::new();

    let mut positions = Vec::new();
    let mut tex_coords = Vec::new();
    let mut normals = Vec::new();
    let mut param_vertices = Vec::new();

    let points = Vec::new();
    let lines = Vec::new();
    let mut polygons = Vec::new();

    let counter = Counter::new(&points, &lines, &polygons);
    let mut group_builder       = counter.hash_map("default".to_string());
    let mut mesh_builder        = counter.hash_map(String::new());
    let mut smoothing_builder   = counter.vec_map();
    let mut merging_builder     = counter.vec_map();

    try!(lex(input, |stmt, args| {
        match stmt {
            // Vertex data
            "v" => positions.push(match f!(args) {
                [x, y, z, w] => f32x4(x, y, z, w),
                [x, y, z] => f32x4(x, y, z, 1.0),
                _ => error!(WrongNumberOfArguments, "Expected 3 or 4 arguments")
            }),
            "vt" => tex_coords.push(match f!(args) {
                [u, v, w] => f32x4(u, v, w, 0.0),
                [u, v] => f32x4(u, v, 0.0, 0.0),
                [u] => f32x4(u, 0.0, 0.0, 0.0),
                _ => error!(WrongNumberOfArguments, "Expected 1, 2 or 3 arguments")
            }),
            "vn" => normals.push(match f!(args) {
                [x, y, z] => f32x4(x, y, z, 0.0),
                _ => error!(WrongNumberOfArguments, "Expected 3 arguments")
            }),
            "vp" => param_vertices.push(match f!(args) {
                [u, v, w] => f32x4(u, v, w, 0.0),
                [u, v] => f32x4(u, v, 1.0, 0.0),
                [u] => f32x4(u, 0.0, 1.0, 0.0),
                _ => error!(WrongNumberOfArguments, "Expected 1, 2 or 3 arguments")
            }),

            // Free-form curve / surface attributes
            "cstype" => {
                let _rational: bool;
                let geometry = match args {
                    ["rat", ty] => { _rational = true; ty }
                    [ty] => { _rational = false; ty }
                    _ => error!(WrongTypeOfArguments, "Expected 'rat xxx' or 'xxx' format")
                };

                match geometry {
                    "bmatrix" => unimplemented!(),
                    "bezier" => unimplemented!(),
                    "bspline" => unimplemented!(),
                    "cardinal" => unimplemented!(),
                    "taylor" => unimplemented!(),
                    _ => error!(WrongTypeOfArguments, "Expected one of 'bmatrix', 'bezier', 'bspline', 'cardinal' and 'taylor'")
                }
            }
            "deg" => match f!(args) {
                [_deg_u, _deg_v]  => unimplemented!(),
                [_deg_u] => unimplemented!(),
                _ => error!(WrongNumberOfArguments, "Expected 1 or 2 arguments")
            },
            "bmat" => unimplemented!(),
            "step" => unimplemented!(),

            // Elements
            "p" => unimplemented!(),
            "l" => unimplemented!(),
            "f" => {
                if args.len() < 3 { error!(WrongNumberOfArguments, "Expected at least 3 arguments") }

                let mut args = args.iter();
                let first = args.next().unwrap();

                macro_rules! m {
                    { $($pat:pat => $name:ident[$exp:expr]),* } => (
                        match s!(first) {
                            $($pat => Polygon::$name({
                                let mut polygon = vec![ $exp ];
                                for param in args {
                                    match s!(param) {
                                        $pat => polygon.push($exp),
                                        _ => unimplemented!()
                                    }
                                }
                                polygon
                            }),)*
                            _ => error!(WrongTypeOfArguments, "Expected '#', '#/#', '#//#' or '#/#/#' format")
                        }
                    )
                }

                polygons.push(m! {
                    [p]        => P[n!(p) - 1],
                    [p, t]     => PT[(n!(p) - 1, n!(t) - 1)],
                    [p, "", u] => PN[(n!(p) - 1, n!(u) - 1)],
                    [p, t, u]  => PTN[(n!(p) - 1, n!(t) - 1, n!(u) - 1)]
                });
            }
            "curv" => unimplemented!(),
            "curv2" => unimplemented!(),
            "surf" => unimplemented!(),

            // Free-form curve / surface body statements
            "parm" => unimplemented!(),
            "trim" => unimplemented!(),
            "hole" => unimplemented!(),
            "scrv" => unimplemented!(),
            "sp" => unimplemented!(),
            "end" => unimplemented!(),

            // Connectivity between free-form surfaces
            "con" => unimplemented!(),

            // Grouping
            "g" => match args {
                [name] => group_builder.start(name.to_string()),
                _ => error!(WrongNumberOfArguments, "Expected group name parameter, but nothing has been supplied")
            },
            "s" => match args {
                ["off"] | ["0"] => smoothing_builder.end(),
                [param] => smoothing_builder.start(n!(param)),
                _ => error!(WrongNumberOfArguments, "Expected only 1 argument")
            },
            "mg" => match args {
                ["off"] | ["0"] => merging_builder.end(),
                [param] => merging_builder.start(n!(param)),
                _ => error!(WrongNumberOfArguments, "Expected only 1 argument")
            },
            "o" => name = match args {
                [] => None,
                args => Some(args.connect(" "))
            },

            // Display / render attributes
            "bevel" => unimplemented!(),
            "c_interp" => unimplemented!(),
            "d_interp" => unimplemented!(),
            "lod" => unimplemented!(),
            "usemtl" => match args {
                [material] => mesh_builder.start(material.to_string()),
                _ => error!(WrongNumberOfArguments, "Expected only 1 argument")
            },
            "mtllib" => {
                let paths: Vec<String> = args.iter().map(|path| path.to_string()).collect();
                material_libraries.push_all(&paths[..]);
            }
            "shadow_obj" => unimplemented!(),
            "trace_obj" => unimplemented!(),
            "ctech" => unimplemented!(),
            "stech" => unimplemented!(),

            // Unexpected statement
            _ => error!(UnexpectedStatement, "Received unknown statement")
        }

        Ok(())
    }));

    group_builder.end();
    mesh_builder.end();
    smoothing_builder.end();
    merging_builder.end();

    Ok(RawObj {
        name: name,
        material_libraries: material_libraries,

        positions: positions,
        tex_coords: tex_coords,
        normals: normals,
        param_vertices: param_vertices,

        points: points,
        lines: lines,
        polygons: polygons,

        groups: group_builder.result,
        meshes: mesh_builder.result,
        smoothing_groups: smoothing_builder.result,
        merging_groups: merging_builder.result
    })
}


/// Counts current total count of parsed `points`, `lines` and `polygons`.
struct Counter {
    points:     *const Vec<Point>,
    lines:      *const Vec<Line>,
    polygons:   *const Vec<Polygon>,
}

impl Counter {
    /// Constructs a new `Counter`.
    fn new(points: *const Vec<Point>, lines: *const Vec<Line>, polygons: *const Vec<Polygon>) -> Self {
        Counter {
            points:     points,
            lines:      lines,
            polygons:   polygons
        }
    }

    /// Returns a current count of parsed `(points, lines, polygons)`.
    fn get(&self) -> (usize, usize, usize) {
        unsafe { ((*self.points).len(), (*self.lines).len(), (*self.polygons).len()) }
    }

    /// Creates a `HashMap<String, Group>` builder which references `self` as counter.
    fn hash_map<'a>(&'a self, input: String) -> GroupBuilder<'a, HashMap<String, Group>, String> {
        let mut result = HashMap::with_capacity(1);
        result.insert(input.clone(), Group::new((0, 0, 0)));

        GroupBuilder {
            counter: self,
            current: Some(input),
            result: result
        }
    }

    /// Creates a `VecMap<Group>` builder which references `self` as counter.
    fn vec_map<'a>(&'a self) -> GroupBuilder<'a, VecMap<Group>, usize> {
        GroupBuilder {
            counter: self,
            current: None,
            result: VecMap::new()
        }
    }
}


/// Helper for creating `groups`, `meshes`, `smoothing_groups` and `merging_groups` member of
/// `Obj`.
struct GroupBuilder<'a, T, K> {
    counter: &'a Counter,
    current: Option<K>, // Some(K) if some group has been started
                        // None    otherwise
    result: T
}

impl<'a, T, K> GroupBuilder<'a, T, K> where
    T: Map<K, Group>,
    K: Clone + Key
{
    /// Starts a group whose name is `input`.
    fn start(&mut self, input: K) {
        let count = self.counter.get();
        if let Some(ref current) = self.current {
            if *current == input { return }
            if self.result[*current].end(count) {
                let res = self.result.remove(&current);
                assert!(res.is_some());
            }
        }
        (|| {
            if let Some(ref mut group) = self.result.get_mut(&input) { group.start(count); return }
            let res = self.result.insert(input.clone(), Group::new(count));
            assert!(res.is_none());
        })();
        self.current = Some(input);
    }

    /// Ends a current group.
    fn end(&mut self) {
        if let Some(ref current) = self.current {
            if self.result[*current].end(self.counter.get()) {
                let result = self.result.remove(current);
                assert!(result.is_some());
            }
        } else { return }
        self.current = None;
    }
}


/// Constant which is used to represent undefined bound of range.
const UNDEFINED: usize = ::std::usize::MAX;

impl Group {
    fn new(count: (usize, usize, usize)) -> Self {
        let mut ret = Group {
            points:     Vec::with_capacity(1),
            lines:      Vec::with_capacity(1),
            polygons:   Vec::with_capacity(1)
        };
        ret.start(count);
        ret
    }

    fn start(&mut self, count: (usize, usize, usize)) {
        self.points.push(Range { start: count.0, end: UNDEFINED });
        self.lines.push(Range { start: count.1, end: UNDEFINED });
        self.polygons.push(Range { start: count.2, end: UNDEFINED })
    }

    /// Closes group, return true if self is empty
    fn end(&mut self, count: (usize, usize, usize)) -> bool {
        end(&mut self.points, count.0);
        end(&mut self.lines, count.1);
        end(&mut self.polygons, count.2);

        fn end(vec: &mut Vec<Range>, end: usize) {
            let last = vec.len() - 1;
            assert_eq!(vec[last].end, UNDEFINED);
            if vec[last].start != end {
                vec[last].end = end;
            } else {
                vec.pop();
            }
        }

        self.points.is_empty() && self.lines.is_empty() && self.polygons.is_empty()
    }
}


/// Custom trait to interface `HashMap` and `VecMap`.
trait Map<K: Key, V: ?Sized> : ::std::ops::IndexMut<K, Output=V> {
    /// Interface of `insert` function.
    fn insert(&mut self, K, V) -> Option<V>;
    /// Interface of `get_mut` function.
    fn get_mut(&mut self, k: &K) -> Option<&mut V>;
    /// Interface of `remove` function.
    fn remove(&mut self, k: &K) -> Option<V>;
}

impl<V> Map<String, V> for HashMap<String, V> {
    fn insert(&mut self, k: String, v: V) -> Option<V> { self.insert(k, v) }
    fn get_mut(&mut self, k: &String) -> Option<&mut V> { self.get_mut(k) }
    fn remove(&mut self, k: &String) -> Option<V> { self.remove(k) }
}

impl<V> Map<usize, V> for VecMap<V> {
    fn insert(&mut self, k: usize, v: V) -> Option<V> { self.insert(k, v) }
    fn get_mut(&mut self, k: &usize) -> Option<&mut V> { self.get_mut(k) }
    fn remove(&mut self, k: &usize) -> Option<V> { self.remove(k) }
}

/// A trait which should be implemented by a type passed into `Key` of `Map`.
trait Key : Eq {}

impl Key for String {}
impl Key for usize {}


/// Low-level Rust binding for `.obj` format.
pub struct RawObj {
    /// Name of the object.
    pub name: Option<String>,
    /// `.mtl` files which required by this object.
    pub material_libraries: Vec<String>,

    /// Position vectors of each vertex.
    pub positions: Vec<f32x4>,
    /// Texture coordinates of each vertex.
    pub tex_coords: Vec<f32x4>,
    /// Normal vectors of each vertex.
    pub normals: Vec<f32x4>,
    /// Parametric vertices.
    pub param_vertices: Vec<f32x4>,

    /// Points which stores the index data of position vectors.
    pub points: Vec<Point>,
    /// Lines which store the index data of vectors.
    pub lines: Vec<Line>,
    /// Polygons which store the index data of vectors.
    pub polygons: Vec<Polygon>,

    /// Groups of multiple geometries.
    pub groups: HashMap<String, Group>,
    /// Geometries which consist in a same material.
    pub meshes: HashMap<String, Group>,
    /// Smoothing groups.
    pub smoothing_groups: VecMap<Group>,
    /// Merging groups.
    pub merging_groups: VecMap<Group>
}

/// The `Point` type which stores the index of the position vector.
pub type Point = usize;

/// The `Line` type.
#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum Line {
    /// A line which contains only the position data of both ends
    P([usize; 2]),
    /// A line which contains both position and texture coordinate data of both ends
    PT([(usize, usize); 2])
}

/// The `Polygon` type.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Polygon {
    /// A polygon which contains only the position data of each vertex.
    P(Vec<usize>),
    /// A polygon which contains both position and texture coordinate data of each vertex.
    PT(Vec<(usize, usize)>),
    /// A polygon which contains both position and normal data of each vertex.
    PN(Vec<(usize, usize)>),
    /// A polygon which contains all position, texture coordinate and normal data of each vertex.
    PTN(Vec<(usize, usize, usize)>)
}

/// A group which contains ranges of points, lines and polygons
#[derive(Clone, Debug)]
pub struct Group {
    /// Multiple range of points
    pub points: Vec<Range>,
    /// Multiple range of lines
    pub lines: Vec<Range>,
    /// Multiple range of polygons
    pub polygons: Vec<Range>
}


/// A struct which represent `[start, end)` range.
#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub struct Range {
    /// The lower bound of the range (inclusive).
    pub start: usize,
    /// The upper bound of the range (exclusive).
    pub end: usize
}
