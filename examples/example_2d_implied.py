import numpy as np
import matplotlib.pyplot as plt
from pytucanos.mesh import get_square, Mesh22, plot_mesh, implied_metric_2d, plot_metric
from pytucanos.remesh import Remesher2dAniso


def get_m(msh):

    x, y = msh.get_coords().T

    hx = 0.1
    h0 = 0.001
    hy = h0 + 2 * (0.1 - h0) * abs(y - 0.5)

    m = np.zeros((x.size, 3))
    m[:, 0] = 1.0 / hx**2
    m[:, 1] = 1.0 / hy**2

    return m


if __name__ == "__main__":

    coords, elems, etags, faces, ftags = get_square()
    msh = Mesh22(coords, elems, etags, faces, ftags)
    msh = msh.split().split()

    # add the missing boundaries, & orient them outwards
    msh.add_boundary_faces()
    # Hilbert renumbering
    msh.reorder_hilbert()

    msh.compute_vertex_to_elems()
    msh.compute_volumes()

    print(msh.n_elems())

    m_i = implied_metric_2d(msh)

    fig, ax = plt.subplots()
    plot_mesh(ax, msh)
    plot_metric(ax, msh, m_i, loc="elem")
    ax.set_title("Implied metric - elem")

    m_i = Remesher2dAniso.elem_data_to_vertex_data_metric(msh, m_i)
    fig, ax = plt.subplots()
    plot_mesh(ax, msh)
    plot_metric(ax, msh, m_i, loc="vertex")
    ax.set_title("Implied metric - vertex")

    plt.show()
