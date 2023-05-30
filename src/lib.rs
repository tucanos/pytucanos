use log::info;
use numpy::{PyArray, PyArray1, PyArray2, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    pyclass, pyfunction, pymethods, pymodule,
    types::{PyModule, PyType},
    wrap_pyfunction, PyResult, Python,
};
use std::collections::HashMap;
use tucanos::{
    geometry::{Geometry, LinearGeometry},
    mesh::SimplexMesh,
    mesh_stl::orient_stl,
    metric::{AnisoMetric2d, AnisoMetric3d, IsoMetric, Metric},
    remesher::{Remesher, RemesherParams, SmoothingType},
    topo_elems::{Edge, Elem, Tetrahedron, Triangle},
    FieldType, Idx, Mesh, Tag,
};

fn to_numpy_1d<T: numpy::Element>(py: Python<'_>, vec: Vec<T>) -> &'_ PyArray1<T> {
    PyArray::from_vec(py, vec)
}

fn to_numpy_2d<T: numpy::Element>(py: Python<'_>, vec: Vec<T>, m: usize) -> &'_ PyArray2<T> {
    let n = vec.len();
    PyArray::from_vec(py, vec).reshape([n / m, m]).unwrap()
}

fn to_numpy_1d_copy<'py, T: numpy::Element>(py: Python<'py>, vec: &[T]) -> &'py PyArray1<T> {
    PyArray::from_slice(py, vec)
}

fn to_numpy_2d_copy<'py, T: numpy::Element>(
    py: Python<'py>,
    vec: &[T],
    m: usize,
) -> &'py PyArray2<T> {
    PyArray::from_slice(py, vec)
        .reshape([vec.len() / m, m])
        .unwrap()
}

macro_rules! create_mesh {
    ($name: ident, $dim: expr, $etype: ident) => {
        #[doc = concat!("Mesh consisting of ", stringify!($etype), " in ", stringify!($dim), "D")]
        #[pyclass]
        pub struct $name {
            mesh: SimplexMesh<$dim, $etype>,
        }
        #[pymethods]
        impl $name {
            /// Create a new mesh from numpy arrays
            /// The data is copied
            #[new]
            pub fn new(
                coords: PyReadonlyArray2<f64>,
                elems: PyReadonlyArray2<Idx>,
                etags: PyReadonlyArray1<Tag>,
                faces: PyReadonlyArray2<Idx>,
                ftags: PyReadonlyArray1<Tag>,
            ) -> PyResult<Self> {
                if coords.shape()[1] != $dim {
                    return Err(PyValueError::new_err("Invalid dimension 1 for coords"));
                }
                let n = elems.shape()[0];
                if elems.shape()[1] != <$etype as Elem>::N_VERTS as usize {
                    return Err(PyValueError::new_err("Invalid dimension 1 for elems"));
                }
                if etags.shape()[0] != n {
                    return Err(PyValueError::new_err("Invalid dimension 0 for etags"));
                }
                let n = faces.shape()[0];

                if faces.shape()[1] != <$etype as Elem>::Face::N_VERTS as usize {
                    return Err(PyValueError::new_err("Invalid dimension 1 for faces"));
                }
                if ftags.shape()[0] != n {
                    return Err(PyValueError::new_err("Invalid dimension 0 for ftags"));
                }

                info!(
                    "Create a {} mesh in {}D",
                    stringify!($etype),
                    stringify!($dim)
                );
                Ok(Self {
                    mesh: SimplexMesh::<$dim, $etype>::new(
                        coords.to_vec().unwrap(),
                        elems.to_vec().unwrap(),
                        etags.to_vec().unwrap(),
                        faces.to_vec().unwrap(),
                        ftags.to_vec().unwrap(),
                    ),
                })
            }

            #[doc = concat!("Read a ", stringify!($name), " from a .mesh(b) file")]
            #[classmethod]
            #[cfg(feature = "libmeshb-sys")]
            pub fn from_meshb(_cls: &PyType, fname: &str) -> PyResult<Self> {
                let res = SimplexMesh::<$dim, $etype>::read_meshb(fname);
                match res {
                    Ok(mesh) => Ok(Self{mesh}),
                    Err(err) => Err(PyRuntimeError::new_err(err.to_string())),
                }
            }

            /// Write the mesh to a .mesh(b) file
            #[cfg(feature = "libmeshb-sys")]
            pub fn write_meshb(&self, fname: &str) -> PyResult<()> {
                self.mesh.write_meshb(fname).map_err(|e| PyRuntimeError::new_err(e.to_string()))
            }

            /// Write a solution to a .sol(b) file
            #[cfg(feature = "libmeshb-sys")]
            pub fn write_solb(&self, fname: &str, arr: PyReadonlyArray2<f64>) -> PyResult<()> {
                self.mesh.write_solb(&arr.to_vec().unwrap(), fname).map_err(|e| PyRuntimeError::new_err(e.to_string()))
            }

            /// Get the number of vertices in the mesh
            #[must_use]
            pub fn n_verts(&self) -> Idx {
                self.mesh.n_verts()
            }

            /// Get the number of vertices in the mesh
            #[must_use]
            pub fn n_elems(&self) -> Idx {
                self.mesh.n_elems()
            }

            /// Get the number of faces in the mesh
            #[must_use]
            pub fn n_faces(&self) -> Idx {
                self.mesh.n_faces()
            }

            /// Get the volume of the mesh
            #[must_use]
            pub fn vol(&self) -> f64 {
                self.mesh.elem_vols().sum()
            }

            /// Get the volume of all the elements
            pub fn vols<'py>(&self, py: Python<'py>) -> &'py PyArray1<f64> {

                let res : Vec<_> = self.mesh.elem_vols().collect();
                to_numpy_1d(py, res)
            }

            /// Compute the vertex-to-element connectivity
            pub fn compute_vertex_to_elems(&mut self) {
                self.mesh.compute_vertex_to_elems();
            }

            /// Clear the vertex-to-element connectivity
            pub fn clear_vertex_to_elems(&mut self) {
                self.mesh.clear_vertex_to_elems();
            }

            /// Compute the face-to-element connectivity
            pub fn compute_face_to_elems(&mut self) {
                self.mesh.compute_face_to_elems();
            }

            /// Clear the face-to-element connectivity
            pub fn clear_face_to_elems(&mut self) {
                self.mesh.clear_face_to_elems();
            }

            /// Compute the element-to-element connectivity
            /// face-to-element connectivity is computed if not available
            pub fn compute_elem_to_elems(&mut self) {
                self.mesh.compute_elem_to_elems();
            }

            /// Clear the element-to-element connectivity
            pub fn clear_elem_to_elems(&mut self) {
                self.mesh.clear_elem_to_elems();
            }

            /// Compute the edges
            pub fn compute_edges(&mut self) {
                self.mesh.compute_edges()
            }

            /// Clear the edges
            pub fn clear_edges(&mut self) {
                self.mesh.clear_edges()
            }

            /// Compute the vertex-to-vertex connectivity
            /// Edges are computed if not available
            pub fn compute_vertex_to_vertices(&mut self) {
                self.mesh.compute_vertex_to_vertices();
            }

            /// Clear the vertex-to-vertex connectivity
            pub fn clear_vertex_to_vertices(&mut self) {
                self.mesh.clear_vertex_to_vertices();
            }

            /// Compute the volume and vertex volumes
            pub fn compute_volumes(&mut self) {
                self.mesh.compute_volumes();
            }

            /// Clear the volume and vertex volumes
            pub fn clear_volumes(&mut self) {
                self.mesh.clear_volumes();
            }

            /// Compute an octree
            pub fn compute_octree(&mut self) {
                self.mesh.compute_octree();
            }

            /// Clear the octree
            pub fn clear_octree(&mut self) {
                self.mesh.clear_octree();
            }

            /// Split all the elements and faces uniformly
            /// NB: vertex and element data is lost
            #[must_use]
            pub fn split(&self) -> Self {
                Self {
                    mesh: self.mesh.split(),
                }
            }

            /// Add the missing boundary faces and make sure that boundary faces are oriented outwards
            /// If internal faces are present, these are keps
            pub fn add_boundary_faces(&mut self) -> Idx {
                self.mesh.add_boundary_faces()
            }

            /// Write a vtk file containing the mesh
            pub fn write_vtk(&self, file_name: &str, vert_data : Option<HashMap<String, PyReadonlyArray2<f64>>> ) -> PyResult<()> {
                let res = if let Some(data) = vert_data {
                    let mut vdata = HashMap::new();
                    for (name, arr) in data.iter() {
                        vdata.insert(name.to_string(), arr.as_slice().unwrap());
                    }
                    self.mesh.write_vtk(file_name, Some(vdata), None)
                } else {
                    self.mesh.write_vtk(file_name, None, None)
                };
                if let Err(res) = res {
                    return Err(PyRuntimeError::new_err(res.to_string()));
                }
                Ok(())
            }

            /// Write a vtk file containing the boundary
            pub fn write_boundary_vtk(&self, file_name: &str) -> PyResult<()> {
                let res = self.mesh.boundary().0.write_vtk(file_name, None, None);
                if let Err(res) = res {
                    return Err(PyRuntimeError::new_err(res.to_string()));
                }
                Ok(())
            }

            #[doc = concat!("Get a copy of the mesh coordinates as a numpy array of shape (# of vertices, ", stringify!($dim), ")")]
            pub fn get_coords<'py>(&mut self, py: Python<'py>) -> &'py PyArray2<f64> {
                to_numpy_2d_copy(py, &self.mesh.coords, $dim)
            }

            /// Get a copy of the element connectivity as a numpy array of shape (# of elements, m)
            pub fn get_elems<'py>(&mut self, py: Python<'py>) -> &'py PyArray2<Idx> {
                to_numpy_2d_copy(py, &self.mesh.elems, <$etype as Elem>::N_VERTS as usize)
            }

            /// Get a copy of the element tags as a numpy array of shape (# of elements)
            #[must_use]
            pub fn get_etags<'py>(&self, py: Python<'py>) -> &'py PyArray1<Tag> {
                to_numpy_1d_copy(py, &self.mesh.etags)
            }

            /// Get a copy of the face connectivity as a numpy array of shape (# of faces, m)
            #[must_use]
            pub fn get_faces<'py>(&self, py: Python<'py>) -> &'py PyArray2<Idx> {
                to_numpy_2d_copy(
                    py,
                    &self.mesh.faces,
                    <$etype as Elem>::Face::N_VERTS as usize,
                )
            }

            /// Get a copy of the face tags as a numpy array of shape (# of faces)
            #[must_use]
            pub fn get_ftags<'py>(&self, py: Python<'py>) -> &'py PyArray1<Tag> {
                to_numpy_1d_copy(py, &self.mesh.ftags)
            }

            /// Reorder the vertices, element and faces using a Hilbert SFC
            pub fn reorder_hilbert<'py>(&mut self, py: Python<'py>) -> PyResult<(&'py PyArray1<Idx>, &'py PyArray1<Idx>, &'py PyArray1<Idx>)>{
                let (new_vertex_indices, new_elem_indices, new_face_indices) = self.mesh.reorder_hilbert();
                Ok(
                    (
                        to_numpy_1d(py, new_vertex_indices),
                        to_numpy_1d(py, new_elem_indices),
                        to_numpy_1d(py, new_face_indices)
                    )
                )

            }

            /// Convert a (scalar or vector) field defined at the element centers (P0) to a field defined at the vertices (P1)
            /// using a weighted average.
            pub fn elem_data_to_vertex_data<'py>(
                &mut self,
                py: Python<'py>,
                arr: PyReadonlyArray2<f64>,
            ) -> PyResult<&'py PyArray2<f64>> {
                if arr.shape()[0] != self.mesh.n_elems() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }

                let res = self.mesh.elem_data_to_vertex_data(arr.as_slice().unwrap());

                if let Err(res) = res {
                    return Err(PyRuntimeError::new_err(res.to_string()));
                }
                Ok(to_numpy_2d(py, res.unwrap(), arr.shape()[1]))
            }

            /// Convert a field (scalar or vector) defined at the vertices (P1) to a field defined at the
            /// element centers (P0)
            pub fn vertex_data_to_elem_data<'py>(
                &mut self,
                py: Python<'py>,
                arr: PyReadonlyArray2<f64>,
            ) -> PyResult<&'py PyArray2<f64>> {
                if arr.shape()[0] != self.mesh.n_verts() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                let res = self.mesh.vertex_data_to_elem_data(arr.as_slice().unwrap());
                Ok(to_numpy_2d(py, res.unwrap(), arr.shape()[1]))
            }

            /// Interpolate a field (scalar or vector) defined at the vertices (P1) to a different mesh
            pub fn interpolate<'py>(
                &mut self,
                py: Python<'py>,
                other: &Self,
                arr: PyReadonlyArray2<f64>,
            ) -> PyResult<&'py PyArray2<f64>> {
                if arr.shape()[0] != self.mesh.n_verts() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                let res = self.mesh.interpolate(&other.mesh, arr.as_slice().unwrap());
                Ok(to_numpy_2d(py, res.unwrap(), arr.shape()[1]))
            }

            /// Smooth a field defined at the mesh vertices using a 1st order least-square approximation
            pub fn smooth<'py>(
                &self,
                py: Python<'py>,
                arr: PyReadonlyArray2<f64>,
                weight_exp: Option<i32>,
            ) -> PyResult<&'py PyArray2<f64>> {
                if arr.shape()[0] != self.mesh.n_verts() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                if arr.shape()[1] != 1 {
                    return Err(PyValueError::new_err("Invalid dimension 1"));
                }

                let res = self
                    .mesh
                    .smooth(arr.as_slice().unwrap(), weight_exp.unwrap_or(2));
                if let Err(res) = res {
                    return Err(PyRuntimeError::new_err(res.to_string()));
                }
                Ok(to_numpy_2d(py, res.unwrap(), arr.shape()[1]))
            }

            /// Compute the gradient of a field defined at the mesh vertices using a 1st order least-square approximation
            pub fn compute_gradient<'py>(
                &self,
                py: Python<'py>,
                arr: PyReadonlyArray2<f64>,
                weight_exp: Option<i32>,
            ) -> PyResult<&'py PyArray2<f64>> {
                if arr.shape()[0] != self.mesh.n_verts() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                if arr.shape()[1] != 1 {
                    return Err(PyValueError::new_err("Invalid dimension 1"));
                }

                let res = self
                    .mesh
                    .gradient(arr.as_slice().unwrap(), weight_exp.unwrap_or(2));
                if let Err(res) = res {
                    return Err(PyRuntimeError::new_err(res.to_string()));
                }
                Ok(to_numpy_2d(
                    py,
                    res.unwrap(),
                    self.mesh.n_comps(FieldType::Vector) as usize,
                ))
            }

            /// Compute the hessian of a field defined at the mesh vertices using a 2nd order least-square approximation
            pub fn compute_hessian<'py>(
                &self,
                py: Python<'py>,
                arr: PyReadonlyArray2<f64>,
                weight_exp: Option<i32>,
            ) -> PyResult<&'py PyArray2<f64>> {
                if arr.shape()[0] != self.mesh.n_verts() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                if arr.shape()[1] != 1 {
                    return Err(PyValueError::new_err("Invalid dimension 1"));
                }

                let res = self
                    .mesh
                    .hessian(arr.as_slice().unwrap(), weight_exp.unwrap_or(2));
                if let Err(res) = res {
                    return Err(PyRuntimeError::new_err(res.to_string()));
                }
                Ok(to_numpy_2d(
                    py,
                    res.unwrap(),
                    self.mesh.n_comps(FieldType::SymTensor) as usize,
                ))
            }

            /// Check that the mesh is valid
            ///  - all elements have a >0 volume
            ///  - all boundary faces are tagged
            ///  - all the faces between different element tags are tagged
            ///  - no other face is tagged
            pub fn check(&self) -> PyResult<()> {
                self.mesh.check().map_err(|e| PyRuntimeError::new_err(e.to_string()))
            }

            /// Compute the topology
            pub fn compute_topology(&mut self) {
                self.mesh.compute_topology();
            }

            /// Clear the topology
            pub fn clear_topology(&mut self) {
                self.mesh.clear_topology();
            }
        }
    };
}

create_mesh!(Mesh33, 3, Tetrahedron);
create_mesh!(Mesh32, 3, Triangle);
create_mesh!(Mesh31, 3, Edge);
create_mesh!(Mesh22, 2, Triangle);
create_mesh!(Mesh21, 2, Edge);

macro_rules! create_geometry {
    ($name: ident, $dim: expr, $etype: ident, $mesh: ident, $geom: ident) => {
        #[doc = concat!("Piecewise linear geometry consisting of ", stringify!($etype), " in ", stringify!($dim), "D")]
        #[pyclass]
        // #[derive(Clone)]
        pub struct $name {
            geom: Option<LinearGeometry<$dim, $etype>>,
        }
        #[pymethods]
        impl $name {
            /// Create a new geometry
            #[new]
            pub fn new(
                mesh: &$mesh,
                geom: Option<&$geom>,
            ) -> Self {

                let mut gmesh = if let Some(geom) = geom {
                    geom.mesh.clone()
                } else {
                    mesh.mesh.boundary().0
                };
                orient_stl(&mesh.mesh, &mut gmesh);
                gmesh.compute_octree();
                let geom = LinearGeometry::new(&mesh.mesh, gmesh).unwrap();

                Self{geom: Some(geom)}
            }

            /// Compute the max distance between the face centers and the geometry normals
            pub fn max_distance(&self, mesh: &$mesh) -> f64 {
                self.geom.as_ref().unwrap().max_distance(&mesh.mesh)
            }

            /// Compute the max angle between the face normals and the geometry normals
            pub fn max_normal_angle(&self, mesh: &$mesh) -> f64 {
                self.geom.as_ref().unwrap().max_normal_angle(&mesh.mesh)
            }
        }
    }
}

create_geometry!(LinearGeometry3d, 3, Triangle, Mesh33, Mesh32);
create_geometry!(LinearGeometry2d, 2, Edge, Mesh22, Mesh21);

#[pymethods]
impl LinearGeometry3d {
    pub fn compute_curvature(&mut self) -> PyResult<()> {
        match &mut self.geom {
            Some(geom) => {
                geom.compute_curvature();
                Ok(())
            }
            None => Err(PyRuntimeError::new_err("Invalid object")),
        }
    }

    pub fn write_curvature_vtk(&self, fname: &str) -> PyResult<()> {
        match &self.geom {
            Some(geom) => geom
                .write_curvature(fname)
                .map_err(|e| PyRuntimeError::new_err(e.to_string())),
            None => Err(PyRuntimeError::new_err("Invalid object")),
        }
    }
}

#[pymethods]
impl Mesh33 {
    /// Extract the boundary faces into a Mesh, and return the indices of the vertices in the
    /// parent mesh
    pub fn boundary<'py>(&self, py: Python<'py>) -> (Mesh32, &'py PyArray1<Idx>) {
        let (bdy, ids) = self.mesh.boundary();
        (Mesh32 { mesh: bdy }, to_numpy_1d(py, ids))
    }

    pub fn implied_metric<'py>(&self, py: Python<'py>) -> PyResult<&'py PyArray2<f64>> {
        let res = self.mesh.implied_metric();

        if let Err(res) = res {
            return Err(PyRuntimeError::new_err(res.to_string()));
        }

        let m: Vec<f64> = res.unwrap().iter().flat_map(|m| m.into_iter()).collect();
        Ok(to_numpy_2d(py, m, 6))
    }

    /// Get a metric defined on all the mesh vertices such that
    ///  - for boundary vertices, the principal directions are aligned with the principal curvature directions
    ///    and the sizes to curvature radius ratio is r_h
    ///  - the metric is entended into the volume with gradation beta
    ///  - if an implied metric is provided, the result is limited to (1/step,step) times the implied metric
    #[allow(clippy::too_many_arguments)]
    pub fn curvature_metric<'py>(
        &self,
        py: Python<'py>,
        geom: &LinearGeometry3d,
        r_h: f64,
        beta: f64,
        implied_metric: Option<PyReadonlyArray2<f64>>,
        step: Option<f64>,
        h_min: Option<f64>,
    ) -> PyResult<&'py PyArray2<f64>> {
        let res = if let Some(implied_metric) = implied_metric {
            let implied_metric: Vec<_> = (0..self.mesh.n_verts())
                .map(|i| AnisoMetric3d::from_slice(implied_metric.as_slice().unwrap(), i))
                .collect();
            self.mesh.curvature_metric(
                geom.geom.as_ref().unwrap(),
                r_h,
                beta,
                Some(&implied_metric),
                step,
            )
        } else {
            self.mesh
                .curvature_metric(geom.geom.as_ref().unwrap(), r_h, beta, None, None)
        };

        if let Err(res) = res {
            return Err(PyRuntimeError::new_err(res.to_string()));
        }
        let mut m = res.unwrap();

        if let Some(h_min) = h_min {
            m.iter_mut()
                .for_each(|x| x.scale_with_bounds(1.0, h_min, f64::MAX));
        }

        let m: Vec<f64> = m.iter().flat_map(|m| m.into_iter()).collect();

        Ok(to_numpy_2d(py, m, 6))
    }
}

#[pymethods]
impl Mesh22 {
    /// Extract the boundary faces into a Mesh, and return the indices of the vertices in the
    /// parent mesh
    pub fn boundary<'py>(&self, py: Python<'py>) -> (Mesh21, &'py PyArray1<Idx>) {
        let (bdy, ids) = self.mesh.boundary();
        (Mesh21 { mesh: bdy }, to_numpy_1d(py, ids))
    }

    pub fn implied_metric<'py>(&self, py: Python<'py>) -> PyResult<&'py PyArray2<f64>> {
        let res = self.mesh.implied_metric();

        if let Err(res) = res {
            return Err(PyRuntimeError::new_err(res.to_string()));
        }

        let m: Vec<f64> = res.unwrap().iter().flat_map(|m| m.into_iter()).collect();
        Ok(to_numpy_2d(py, m, 3))
    }
}

/// Read a solution stored in a .sol(b) file
#[pyfunction]
#[cfg(feature = "libmeshb-sys")]
pub fn read_solb<'py>(py: Python<'py>, fname: &str) -> PyResult<&'py PyArray2<f64>> {
    let res = tucanos::meshb_io::read_solb(fname);
    match res {
        Ok((sol, m)) => Ok(to_numpy_2d(py, sol, m)),
        Err(err) => Err(PyRuntimeError::new_err(err.to_string())),
    }
}

macro_rules! create_remesher {
    ($name: ident, $dim: expr, $etype: ident, $metric: ident, $mesh: ident, $geom: ident) => {
        #[doc = concat!("Remesher for a meshes consisting of ", stringify!($etype), " in ", stringify!($dim), "D")]
        #[doc = concat!("using ", stringify!($metric), " as metric and a piecewise linear representation of the geometry")]
        #[pyclass]
        pub struct $name {
            remesher: Remesher<$dim, $etype, $metric, LinearGeometry<$dim, <$etype as Elem>::Face>>,
        }

        #[doc = concat!("Create a remesher from a ", stringify!($mesh), " and a ",stringify!($metric) ," metric defined at the mesh vertices")]
        #[doc = concat!("A piecewise linear representation of the geometry is used, either from the ", stringify!($geom), " given or otherwise from the mesh boundary.")]
        #[pymethods]
        impl $name {
            #[new]
            pub fn new(
                mesh: &$mesh,
                geometry: &mut $geom,
                m: PyReadonlyArray2<f64>,
            ) -> PyResult<Self> {
                if m.shape()[0] != mesh.mesh.n_verts() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                if m.shape()[1] != $metric::N as usize {
                    return Err(PyValueError::new_err("Invalid dimension 1"));
                }

                let m = m.as_slice()?;
                let m: Vec<_> = (0..mesh.n_verts())
                    .map(|i| $metric::from_slice(&m, i))
                    .collect();

                let remesher = Remesher::new(&mesh.mesh, &m, geometry.geom.take().unwrap());
                if let Err(res) = remesher {
                    return Err(PyRuntimeError::new_err(res.to_string()));
                }
                Ok(Self {remesher: remesher.unwrap()})
            }

            /// Convert a Hessian to the optimal metric using a Lp norm.
            #[classmethod]
            pub fn hessian_to_metric<'py>(
                _cls: &PyType,
                py: Python<'py>,
                mesh: &$mesh,
                m: PyReadonlyArray2<f64>,
                p: Option<Idx>,
            ) -> PyResult<&'py PyArray2<f64>> {
                if m.shape()[0] != mesh.mesh.n_verts() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                if m.shape()[1] != <$metric as Metric<$dim>>::N {
                    return Err(PyValueError::new_err("Invalid dimension 1"));
                }

                let mut res = Vec::with_capacity(m.shape()[0] * m.shape()[1]);
                let m = m.as_slice().unwrap();

                let exponent = if let Some(p) = p {
                    -2.0 / (2.0 * p as f64 + $dim as f64)
                } else {
                    0.0
                };

                for i_vert in 0..mesh.mesh.n_verts() {
                    let mut m_v = $metric::from_slice(m, i_vert);
                    let scale = f64::powf(m_v.vol(), exponent);
                    if !scale.is_nan() {
                        m_v.scale(scale);
                    }
                    res.extend(m_v.into_iter());
                }

                return Ok(to_numpy_2d(py, res, <$metric as Metric<$dim>>::N));
            }

            /// Scale a metric field to reach the desired (ideal) number of elements using min / max bounds on the cell size
            #[classmethod]
            #[allow(clippy::too_many_arguments)]
            pub fn scale_metric<'py>(
                _cls: &PyType,
                py: Python<'py>,
                mesh: &$mesh,
                m: PyReadonlyArray2<f64>,
                h_min: f64,
                h_max: f64,
                n_elems: Idx,
                max_iter: Option<Idx>,
                fixed_m: Option<PyReadonlyArray2<f64>>,
            ) -> PyResult<&'py PyArray2<f64>> {
                if m.shape()[0] != mesh.mesh.n_verts() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                if m.shape()[1] != <$metric as Metric<$dim>>::N {
                    return Err(PyValueError::new_err("Invalid dimension 1"));
                }

                let m = m.as_slice().unwrap();

                let mut m: Vec<_> = (0..mesh.mesh.n_verts())
                    .map(|i| $metric::from_slice(m, i))
                    .collect();
                let res = if let Some(fixed_m) = fixed_m {
                    let fixed_m = (0..mesh.mesh.n_verts())
                        .map(|i| $metric::from_slice(fixed_m.as_slice().unwrap(), i))
                        .collect::<Vec<_>>();
                    mesh.mesh
                        .scale_metric(&mut m, Some(&fixed_m), h_min, h_max, n_elems, max_iter.unwrap_or(10))
                } else {
                    mesh.mesh
                        .scale_metric(&mut m, None, h_min, h_max, n_elems, max_iter.unwrap_or(10))
                };
                if let Err(res) = res {
                    return Err(PyRuntimeError::new_err(res.to_string()));
                }

                let m: Vec<_> = m.iter().cloned().flatten().collect();
                return Ok(to_numpy_2d(py, m, <$metric as Metric<$dim>>::N));
            }

            /// Smooth a metric field
            #[classmethod]
            pub fn smooth_metric<'py>(
                _cls: &PyType,
                py: Python<'py>,
                mesh: &$mesh,
                m: PyReadonlyArray2<f64>,
            ) -> PyResult<&'py PyArray2<f64>> {
                if m.shape()[0] != mesh.mesh.n_verts() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                if m.shape()[1] != <$metric as Metric<$dim>>::N {
                    return Err(PyValueError::new_err("Invalid dimension 1"));
                }

                let m = m.as_slice().unwrap();
                let m: Vec<_> = (0..mesh.mesh.n_verts())
                    .map(|i| $metric::from_slice(m, i))
                    .collect();
                let m = mesh.mesh.smooth_metric(&m);
                if let Err(m) = m {
                    return Err(PyRuntimeError::new_err(m.to_string()));
                }

                let m: Vec<_> = m.unwrap().iter().cloned().flatten().collect();

                return Ok(to_numpy_2d(py, m, <$metric as Metric<$dim>>::N));
            }

            /// Apply a maximum gradation to a metric field
            #[classmethod]
            pub fn apply_metric_gradation<'py>(
                _cls: &PyType,
                py: Python<'py>,
                mesh: &$mesh,
                m: PyReadonlyArray2<f64>,
                beta: f64,
                n_iter: Idx,
            ) -> PyResult<&'py PyArray2<f64>> {
                if m.shape()[0] != mesh.mesh.n_verts() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                if m.shape()[1] != <$metric as Metric<$dim>>::N {
                    return Err(PyValueError::new_err("Invalid dimension 1"));
                }

                let m = m.as_slice().unwrap();
                let mut m: Vec<_> = (0..mesh.mesh.n_verts())
                    .map(|i| $metric::from_slice(m, i))
                    .collect();
                let res = mesh.mesh.apply_metric_gradation(&mut m, beta, n_iter);
                match res {
                    Ok(_) => {
                        let m: Vec<_> = m.iter().cloned().flatten().collect();

                        return Ok(to_numpy_2d(py, m, <$metric as Metric<$dim>>::N));
                    }
                    Err(res) => {
                        return Err(PyRuntimeError::new_err(res.to_string()));
                    }
                }
            }

            /// Convert a metic field defined at the element centers (P0) to a field defined at the vertices (P1)
            /// using a weighted average.
            #[classmethod]
            pub fn elem_data_to_vertex_data_metric<'py>(
                _cls: &PyType,
                py: Python<'py>,
                mesh: &$mesh,
                m: PyReadonlyArray2<f64>,
            ) -> PyResult<&'py PyArray2<f64>> {
                if m.shape()[0] != mesh.mesh.n_elems() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                if m.shape()[1] != <$metric as Metric<$dim>>::N {
                    return Err(PyValueError::new_err("Invalid dimension 1"));
                }

                let m = m.as_slice().unwrap();
                let m: Vec<_> = (0..mesh.mesh.n_verts())
                    .map(|i| $metric::from_slice(m, i))
                    .collect();
                let res = mesh.mesh.elem_data_to_vertex_data_metric::<$metric>(&m);
                match res {
                    Ok(res) => {
                        let res: Vec<_> = res.iter().cloned().flatten().collect();
                        return Ok(to_numpy_2d(py, res, <$metric as Metric<$dim>>::N));
                    }
                    Err(res) => {
                        return Err(PyRuntimeError::new_err(res.to_string()));
                    }
                }
            }

            /// Convert a metric field defined at the vertices (P1) to a field defined at the
            /// element centers (P0)
            #[classmethod]
            pub fn vertex_data_to_elem_data_metric<'py>(
                _cls: &PyType,
                py: Python<'py>,
                mesh: &$mesh,
                m: PyReadonlyArray2<f64>,
            ) -> PyResult<&'py PyArray2<f64>> {
                if m.shape()[0] != mesh.mesh.n_verts() as usize {
                    return Err(PyValueError::new_err("Invalid dimension 0"));
                }
                if m.shape()[1] != <$metric as Metric<$dim>>::N {
                    return Err(PyValueError::new_err("Invalid dimension 1"));
                }

                let m = m.as_slice().unwrap();
                let m: Vec<_> = (0..mesh.mesh.n_verts())
                    .map(|i| $metric::from_slice(m, i))
                    .collect();
                let res = mesh.mesh.vertex_data_to_elem_data_metric::<$metric>(&m);
                match res {
                    Ok(res) => {
                        let res: Vec<_> = res.iter().cloned().flatten().collect();
                        return Ok(to_numpy_2d(py, res, <$metric as Metric<$dim>>::N));
                    }
                    Err(res) => {
                        return Err(PyRuntimeError::new_err(res.to_string()));
                    }
                }
            }

            /// Compute the min/max sizes, max anisotropy and complexity of a metric
            #[classmethod]
            pub fn metric_info(
                _cls: &PyType,
                mesh: &$mesh,
                m: PyReadonlyArray2<f64>,
            ) -> (f64, f64, f64, f64) {
                let m = m.as_slice().unwrap();
                let m: Vec<_> = (0..mesh.mesh.n_verts())
                    .map(|i| $metric::from_slice(m, i))
                    .collect();
                mesh.mesh.metric_info(&m)
            }

            /// Check that the mesh is valid
            pub fn check(&self) -> PyResult<()> {
                let res = self.remesher.check();
                if let Err(res) = res {
                    return Err(PyRuntimeError::new_err(res.to_string()));
                }

                Ok(())
            }

            /// Estimate the complexity (ideal number of elements)
            pub fn complexity(&self) -> f64 {
                self.remesher.complexity()
            }

            #[doc = concat!("Get the mesh as a ", stringify!($mesh))]
            #[must_use]
            pub fn to_mesh(&self, only_bdy_faces: Option<bool>) -> $mesh {
                $mesh {
                    mesh: self.remesher.to_mesh(only_bdy_faces.unwrap_or(false)),
                }
            }

            /// Get the number of vertices
            #[must_use]
            pub fn n_verts(&self) -> Idx {
                self.remesher.n_verts()
            }

            /// Get the number of elements
            #[must_use]
            pub fn n_elems(&self) -> Idx {
                self.remesher.n_elems()
            }

            /// Get the number of edges
            #[must_use]
            pub fn n_edges(&self) -> Idx {
                self.remesher.n_edges()
            }

            /// Perform a remeshing iteration
            #[allow(clippy::too_many_arguments)]
            pub fn remesh(
                &mut self,
                num_iter:Option< u32>,
                two_steps: Option<bool>,
                split_max_iter:Option< u32>,
                split_min_l_rel:Option< f64>,
                split_min_l_abs:Option< f64>,
                split_min_q_rel:Option< f64>,
                split_min_q_abs:Option< f64>,
                collapse_max_iter:Option< u32>,
                collapse_max_l_rel:Option< f64>,
                collapse_max_l_abs:Option< f64>,
                collapse_min_q_rel:Option< f64>,
                collapse_min_q_abs:Option< f64>,
                swap_max_iter:Option< u32>,
                swap_max_l_rel:Option< f64>,
                swap_max_l_abs:Option< f64>,
                swap_min_l_rel:Option< f64>,
                swap_min_l_abs:Option< f64>,
                smooth_iter:Option< u32>,
                smooth_type: Option<&str>,
                max_angle:Option< f64>,
            ) {
                let smooth_type = smooth_type.unwrap_or("laplacian");
                let smooth_type = if smooth_type == "laplacian" {
                    SmoothingType::Laplacian2
                } else if smooth_type == "nlopt" {
                    unreachable!()
                } else {
                    SmoothingType::Avro
                };

                let default_params = RemesherParams::default();

                let params = RemesherParams {
                    num_iter: num_iter.unwrap_or(default_params.num_iter),
                    two_steps: two_steps.unwrap_or(default_params.two_steps),
                    split_max_iter: split_max_iter.unwrap_or(default_params.split_max_iter),
                    split_min_l_rel: split_min_l_rel.unwrap_or(default_params.split_min_l_rel),
                    split_min_l_abs: split_min_l_abs.unwrap_or(default_params.split_min_l_abs),
                    split_min_q_rel: split_min_q_rel.unwrap_or(default_params.split_min_q_rel),
                    split_min_q_abs: split_min_q_abs.unwrap_or(default_params.split_min_q_abs),
                    collapse_max_iter: collapse_max_iter.unwrap_or(default_params.collapse_max_iter),
                    collapse_max_l_rel: collapse_max_l_rel.unwrap_or(default_params.collapse_max_l_rel),
                    collapse_max_l_abs: collapse_max_l_abs.unwrap_or(default_params.collapse_max_l_abs),
                    collapse_min_q_rel: collapse_min_q_rel.unwrap_or(default_params.collapse_min_q_rel),
                    collapse_min_q_abs: collapse_min_q_abs.unwrap_or(default_params.collapse_min_q_abs),
                    swap_max_iter: swap_max_iter.unwrap_or(default_params.swap_max_iter),
                    swap_max_l_rel: swap_max_l_rel.unwrap_or(default_params.swap_max_l_rel),
                    swap_max_l_abs: swap_max_l_abs.unwrap_or(default_params.swap_max_l_abs),
                    swap_min_l_rel: swap_min_l_rel.unwrap_or(default_params.swap_min_l_rel),
                    swap_min_l_abs: swap_min_l_abs.unwrap_or(default_params.swap_min_l_abs),
                    smooth_iter: smooth_iter.unwrap_or(default_params.smooth_iter),
                    smooth_type,
                    max_angle: max_angle.unwrap_or(default_params.max_angle),
                };
                self.remesher.remesh(params);
            }

            /// Get the element qualities as a numpy array of size (# or elements)
            #[must_use]
            pub fn qualities<'py>(&self, py: Python<'py>) -> &'py PyArray1<f64> {
                to_numpy_1d(py, self.remesher.qualities())
            }

            /// Get the element lengths (in metric space) as a numpy array of size (# or edges)
            #[must_use]
            pub fn lengths<'py>(&self, py: Python<'py>) -> &'py PyArray1<f64> {
                to_numpy_1d(py, self.remesher.lengths())
            }

            /// Get the infomation about the remeshing steps performed in remesh() as a json string
            pub fn stats_json(&self) -> String {
                self.remesher.stats_json()
            }
        }
    };
}

type IsoMetric2d = IsoMetric<2>;
type IsoMetric3d = IsoMetric<3>;
create_remesher!(
    Remesher2dIso,
    2,
    Triangle,
    IsoMetric2d,
    Mesh22,
    LinearGeometry2d
);
create_remesher!(
    Remesher2dAniso,
    2,
    Triangle,
    AnisoMetric2d,
    Mesh22,
    LinearGeometry2d
);
create_remesher!(
    Remesher3dIso,
    3,
    Tetrahedron,
    IsoMetric3d,
    Mesh33,
    LinearGeometry3d
);
create_remesher!(
    Remesher3dAniso,
    3,
    Tetrahedron,
    AnisoMetric3d,
    Mesh33,
    LinearGeometry3d
);

/// Python bindings for pytucanos
#[pymodule]
#[pyo3(name = "_pytucanos")]
fn pytucanos(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    pyo3_log::init();
    m.add_class::<Mesh33>()?;
    m.add_class::<Mesh32>()?;
    m.add_class::<Mesh31>()?;
    m.add_class::<Mesh22>()?;
    m.add_class::<Mesh21>()?;
    m.add_class::<LinearGeometry2d>()?;
    m.add_class::<LinearGeometry3d>()?;
    #[cfg(feature = "libmeshb-sys")]
    m.add_function(wrap_pyfunction!(read_solb, m)?)?;
    m.add_class::<Remesher2dIso>()?;
    m.add_class::<Remesher2dAniso>()?;
    m.add_class::<Remesher3dIso>()?;
    m.add_class::<Remesher3dAniso>()?;
    #[cfg(not(feature = "libmeshb-sys"))]
    m.add("HAVE_MESHB", false)?;
    #[cfg(feature = "libmeshb-sys")]
    m.add("HAVE_MESHB", true)?;
    Ok(())
}
