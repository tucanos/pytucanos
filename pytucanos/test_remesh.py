import numpy as np
import unittest
from .mesh import (
    Mesh21,
    Mesh22,
    Mesh32,
    Mesh33,
    get_boundary_2d,
    get_boundary_3d,
    get_square,
    get_cube,
)
from .remesh import Remesher2dIso, Remesher2dAniso


class TestRemesh(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import logging

        logging.disable(logging.DEBUG)

    def test_2d_iso(self):

        coords, elems, etags, faces, ftags = get_square()
        msh = Mesh22(coords, elems, etags, faces, ftags)

        h = 0.1 * np.ones(msh.n_verts()).reshape((-1, 1))

        remesher = Remesher2dIso(msh, h)
        remesher.remesh()

        msh = remesher.to_mesh()

        self.assertTrue(np.allclose(msh.vol(), 1.0))
        etags = np.unique(msh.get_etags())
        etags.sort()
        self.assertTrue(np.array_equal(etags, [1, 2]))
        ftags = np.unique(msh.get_ftags())
        ftags.sort()
        self.assertTrue(np.array_equal(ftags, [1, 2, 3, 4, 5]))

        self.assertGreater(msh.n_verts(), 100)
        self.assertLess(msh.n_verts(), 200)

    def test_2d_iso_circle(self):

        coords, elems, etags, faces, ftags = get_square()
        msh = Mesh22(coords, elems, etags, faces, ftags)

        n = 3
        theta = 0.25 * np.pi + np.linspace(0, 2 * np.pi, 4 * n + 1)
        r = 0.5 * 2**0.5
        x = 0.5 + r * np.cos(theta)
        y = 0.5 + r * np.sin(theta)
        coords = np.stack([x, y], axis=-1)

        idx = np.arange(4 * n, dtype=np.uint32)
        elems = np.stack(
            [idx, idx + 1],
            axis=-1,
        )
        elems[-1, 1] = 0
        etags = np.zeros(4 * n, dtype=np.int16)
        etags[0 * n : 1 * n] = 3
        etags[1 * n : 2 * n] = 4
        etags[2 * n : 3 * n] = 1
        etags[3 * n : 4 * n] = 2

        elems = np.vstack(
            [
                elems,
                np.array(
                    [
                        [2 * n, 0],
                    ],
                    dtype=np.uint32,
                ),
            ]
        )
        etags = np.append(
            etags,
            np.array(
                [5],
                dtype=np.int16,
            ),
        )

        faces = np.zeros((0, 1), dtype=np.uint32)
        ftags = np.zeros(0, dtype=np.int16)
        geom = Mesh21(coords, elems, etags, faces, ftags)

        h = 0.1 * np.ones(msh.n_verts()).reshape((-1, 1))

        remesher = Remesher2dIso(msh, h, geometry=geom)
        remesher.remesh()

        msh = remesher.to_mesh()

        self.assertGreater(msh.vol(), 0.9 * 0.5 * np.pi)
        self.assertLess(msh.vol(), 1.1 * 0.5 * np.pi)

        etags = np.unique(msh.get_etags())
        etags.sort()
        self.assertTrue(np.array_equal(etags, [1, 2]))
        ftags = np.unique(msh.get_ftags())
        ftags.sort()
        self.assertTrue(np.array_equal(ftags, [1, 2, 3, 4, 5]))

        self.assertGreater(msh.n_verts(), 100 * 0.9 * msh.vol())
        self.assertLess(msh.n_verts(), 200 * msh.vol())

    def test_2d_aniso(self):

        coords, elems, etags, faces, ftags = get_square()
        msh = Mesh22(coords, elems, etags, faces, ftags)

        hx = 0.3
        hy = 0.03
        m = np.zeros((msh.n_verts(), 3))
        m[:, 0] = 1.0 / hx**2
        m[:, 1] = 1.0 / hy**2

        remesher = Remesher2dAniso(msh, m)
        remesher.remesh(num_iter=4, split_constrain_q=0.5)

        msh = remesher.to_mesh()

        self.assertTrue(np.allclose(msh.vol(), 1.0))
        etags = np.unique(msh.get_etags())
        etags.sort()
        self.assertTrue(np.array_equal(etags, [1, 2]))
        ftags = np.unique(msh.get_ftags())
        ftags.sort()
        self.assertTrue(np.array_equal(ftags, [1, 2, 3, 4, 5]))

        c = remesher.complexity()
        self.assertGreater(msh.n_verts(), 0.9 * c)
        self.assertLess(msh.n_verts(), 2.5 * c)

        self.assertTrue(np.allclose(c, 4.0 / 3.0**0.5 / (0.3 * 0.03)))
