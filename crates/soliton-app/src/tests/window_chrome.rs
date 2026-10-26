use super::resize_edge;
use gpui::{ResizeEdge, Tiling, point, px, size};

#[test]
fn resize_border_distinguishes_edges_corners_and_content() {
    let size = size(px(800.), px(600.));
    for (x, y, expected) in [
        (0., 0., Some(ResizeEdge::TopLeft)),
        (799., 0., Some(ResizeEdge::TopRight)),
        (0., 599., Some(ResizeEdge::BottomLeft)),
        (799., 599., Some(ResizeEdge::BottomRight)),
        (400., 0., Some(ResizeEdge::Top)),
        (400., 599., Some(ResizeEdge::Bottom)),
        (0., 300., Some(ResizeEdge::Left)),
        (799., 300., Some(ResizeEdge::Right)),
        (10., 10., None),
        (400., 300., None),
        (-1., 0., None),
        (0., -1., None),
        (800., 300., None),
        (400., 600., None),
    ] {
        assert_eq!(
            resize_edge(point(px(x), px(y)), size, Tiling::default()),
            expected,
            "position ({x}, {y})"
        );
    }
}

#[test]
fn tiled_edges_cannot_start_a_resize_but_free_edges_still_can() {
    let size = size(px(800.), px(600.));
    let tiling = Tiling {
        top: true,
        left: true,
        ..Default::default()
    };
    assert_eq!(resize_edge(point(px(0.), px(0.)), size, tiling), None);
    assert_eq!(
        resize_edge(point(px(799.), px(0.)), size, tiling),
        Some(ResizeEdge::Right)
    );
    assert_eq!(
        resize_edge(point(px(0.), px(599.)), size, tiling),
        Some(ResizeEdge::Bottom)
    );
    assert_eq!(
        resize_edge(point(px(799.), px(599.)), size, tiling),
        Some(ResizeEdge::BottomRight)
    );
    for (x, y) in [(0., 0.), (799., 0.), (0., 599.), (799., 599.)] {
        assert_eq!(
            resize_edge(point(px(x), px(y)), size, Tiling::tiled()),
            None
        );
    }
}
