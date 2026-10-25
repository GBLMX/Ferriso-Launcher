//! Small, dependency-free line icons drawn with iced's canvas.

use std::cell::RefCell;

use ferriso_gui::iced::{
    Color, Element, Length, Point, Radians, Rectangle, Size, Theme,
    widget::canvas::{
        self, Canvas, Frame, Geometry, LineCap, LineJoin, Path, Stroke,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconKind {
    Home,
    Browse,
    Library,
    Account,
    Skins,
    Hosting,
    Settings,
    Refresh,
    Close,
    Back,
    Search,
    Plus,
    Check,
    Download,
    Dot,
    ChevronRight,
}

pub fn icon<'a, Message: 'a>(
    kind: IconKind,
    size: f32,
    color: Color,
) -> Element<'a, Message> {
    Canvas::new(Icon { kind, color })
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

struct Icon {
    kind: IconKind,
    color: Color,
}

impl<Message> canvas::Program<Message> for Icon {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &ferriso_gui::iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: ferriso_gui::iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let frame = {
            let frame = RefCell::new(Frame::new(renderer, bounds.size()));
            let scale = bounds.width.min(bounds.height) / 24.0;
            let p = |x: f32, y: f32| Point::new(x * scale, y * scale);
            let stroke = Stroke::default()
                .with_color(self.color)
                .with_width((1.8 * scale).max(1.0))
                .with_line_cap(LineCap::Round)
                .with_line_join(LineJoin::Round);
            let stroke_path = |path: &Path| {
                frame.borrow_mut().stroke(path, stroke);
            };
            let fill_path = |path: &Path, color: Color| {
                frame.borrow_mut().fill(path, color);
            };
            let line = |from: Point, to: Point| {
                stroke_path(&Path::line(from, to));
            };
            let polyline = |points: &[(f32, f32)]| {
                let path = Path::new(|builder| {
                    let mut points = points.iter();
                    if let Some((x, y)) = points.next() {
                        builder.move_to(p(*x, *y));
                    }
                    for (x, y) in points {
                        builder.line_to(p(*x, *y));
                    }
                });
                stroke_path(&path);
            };

            match self.kind {
                IconKind::Home => {
                    polyline(&[(3.0, 11.0), (12.0, 3.5), (21.0, 11.0)]);
                    polyline(&[
                        (6.0, 9.5),
                        (6.0, 20.0),
                        (18.0, 20.0),
                        (18.0, 9.5),
                    ]);
                    polyline(&[
                        (10.0, 20.0),
                        (10.0, 14.0),
                        (14.0, 14.0),
                        (14.0, 20.0),
                    ]);
                }
                IconKind::Browse => {
                    stroke_path(&Path::circle(p(12.0, 12.0), 8.0 * scale));
                    fill_path(
                        &Path::new(|builder| {
                            builder.move_to(p(15.5, 8.5));
                            builder.line_to(p(13.0, 13.0));
                            builder.line_to(p(8.5, 15.5));
                            builder.line_to(p(11.0, 11.0));
                            builder.close();
                        }),
                        Color {
                            a: 0.18,
                            ..self.color
                        },
                    );
                    polyline(&[
                        (15.5, 8.5),
                        (13.0, 13.0),
                        (8.5, 15.5),
                        (11.0, 11.0),
                        (15.5, 8.5),
                    ]);
                }
                IconKind::Library => {
                    for x in [5.0, 10.0, 15.0] {
                        stroke_path(&Path::line(p(x, 4.5), p(x + 4.0, 4.5)));
                        stroke_path(&Path::line(
                            p(x + 4.0, 4.5),
                            p(x + 4.0, 19.5),
                        ));
                        stroke_path(&Path::line(p(x + 4.0, 19.5), p(x, 19.5)));
                    }
                    polyline(&[(3.0, 21.0), (21.0, 21.0)]);
                }
                IconKind::Account => {
                    stroke_path(&Path::circle(p(12.0, 8.0), 3.5 * scale));
                    polyline(&[
                        (4.8, 21.0),
                        (5.5, 17.0),
                        (8.5, 14.5),
                        (15.5, 14.5),
                        (18.5, 17.0),
                        (19.2, 21.0),
                    ]);
                }
                IconKind::Skins => {
                    polyline(&[
                        (8.0, 4.0),
                        (10.0, 6.0),
                        (14.0, 6.0),
                        (16.0, 4.0),
                        (21.0, 8.0),
                        (19.0, 11.0),
                        (17.0, 9.5),
                        (17.0, 20.0),
                        (7.0, 20.0),
                        (7.0, 9.5),
                        (5.0, 11.0),
                        (3.0, 8.0),
                        (8.0, 4.0),
                    ]);
                }
                IconKind::Hosting => {
                    for y in [4.0, 11.0] {
                        stroke_path(&Path::rectangle(
                            p(4.0, y),
                            Size::new(16.0 * scale, 6.0 * scale),
                        ));
                        fill_path(
                            &Path::circle(p(18.0, y + 3.0), 0.9 * scale),
                            self.color,
                        );
                    }
                    polyline(&[(4.0, 20.0), (20.0, 20.0)]);
                }
                IconKind::Settings => {
                    for (y, knob) in [(7.0, 9.0), (12.0, 15.0), (17.0, 7.0)] {
                        line(p(4.0, y), p(20.0, y));
                        fill_path(
                            &Path::circle(p(knob, y), 2.0 * scale),
                            self.color,
                        );
                    }
                }
                IconKind::Refresh => {
                    let path = Path::new(|builder| {
                        builder.arc(canvas::path::Arc {
                            center: p(12.0, 12.0),
                            radius: 7.0 * scale,
                            start_angle: Radians(-0.6),
                            end_angle: Radians(4.9),
                        });
                    });
                    stroke_path(&path);
                    polyline(&[(17.0, 5.0), (19.0, 9.0), (15.0, 10.0)]);
                }
                IconKind::Close => {
                    line(p(6.0, 6.0), p(18.0, 18.0));
                    line(p(18.0, 6.0), p(6.0, 18.0));
                }
                IconKind::Back => {
                    polyline(&[(15.0, 5.0), (8.0, 12.0), (15.0, 19.0)]);
                }
                IconKind::Search => {
                    stroke_path(&Path::circle(p(10.5, 10.5), 6.0 * scale));
                    line(p(15.0, 15.0), p(20.0, 20.0));
                }
                IconKind::Plus => {
                    line(p(12.0, 5.0), p(12.0, 19.0));
                    line(p(5.0, 12.0), p(19.0, 12.0));
                }
                IconKind::Check => {
                    polyline(&[(5.0, 12.5), (10.0, 17.0), (19.0, 7.0)]);
                }
                IconKind::Download => {
                    line(p(12.0, 4.0), p(12.0, 15.0));
                    polyline(&[(7.5, 11.0), (12.0, 15.5), (16.5, 11.0)]);
                    polyline(&[(5.0, 19.5), (19.0, 19.5)]);
                }
                IconKind::Dot => {
                    fill_path(
                        &Path::circle(p(12.0, 12.0), 4.0 * scale),
                        self.color,
                    );
                }
                IconKind::ChevronRight => {
                    polyline(&[(9.0, 5.0), (16.0, 12.0), (9.0, 19.0)]);
                }
            }

            frame
        };

        vec![frame.into_inner().into_geometry()]
    }
}
