use ramune::{Color, Event, GameBuilder};

fn main() {
    let (game, _) = GameBuilder::new().build();

    game.poll(move |e| match e {
        Event::Draw(g) => {
            g.clear(Color::CORNFLOWER_BLUE);
            {
                let mut inner = g.push();
                inner.draw_rect(50., 50., 50., 50.);
            }
        }
        _ => {}
    });
}
