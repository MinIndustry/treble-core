#[test]
fn test_system() {
    let mut filters = vec![];

    info!("Creating a source pipe");
    let source = Rc::new(RefCell::new(Pipe::new()));

    let second_source = Rc::new(RefCell::new(Pipe::new()));

    let inbetween = Rc::new(RefCell::new(Pipe::new()));

    info!("Creating a sink pipe");
    let sink = Rc::new(RefCell::new(Pipe::new()));

    info!("Creating a filter and connecting to source and sink pipes");
    let mut filter = Filter::new(Box::new(SumCombinationFilter));
    filter.add_source(Rc::clone(&source));
    filter.add_source(Rc::clone(&second_source));
    filter.add_sink(Rc::clone(&inbetween));

    let mut filter2 = Filter::new(Box::new(DoubleFilter));
    filter2.add_source(Rc::clone(&inbetween));
    filter2.add_sink(Rc::clone(&sink));

    filters.push(filter);
    filters.push(filter2);

    let system = PFSystem {
        filters,
        sources: vec![source, second_source],
        sinks: vec![sink],
    };

    for x in 0..100 {
        system.push(x % 2, x as f32);
    }

    let mut results = Vec::new();
    loop {
        system.run();
        if let Some(val) = system.get_sink(0).borrow_mut().pop() {
            results.push(val)
        }

        if results.len() == 50 {
            break;
        }
    }

    for (id, val) in results.iter().enumerate() {
        assert_eq!(
            *val,
            2.0 * (2 * (2 * id) + 1) as f32,
            "Values do not match !"
        );
    }
}

#[test]
fn stress_test() {
    let mut filters = vec![];

    info!("Creating a source pipe");
    let source = Rc::new(RefCell::new(Pipe::new()));

    let second_source = Rc::new(RefCell::new(Pipe::new()));

    let inbetween = Rc::new(RefCell::new(Pipe::new()));

    info!("Creating a sink pipe");
    let sink = Rc::new(RefCell::new(Pipe::new()));

    info!("Creating a filter and connecting to source and sink pipes");
    let mut filter = Filter::new(Box::new(SumCombinationFilter));
    filter.add_source(Rc::clone(&source));
    filter.add_source(Rc::clone(&second_source));
    filter.add_sink(Rc::clone(&inbetween));

    let mut filter2 = Filter::new(Box::new(DoubleFilter));
    filter2.add_source(Rc::clone(&inbetween));
    filter2.add_sink(Rc::clone(&sink));

    filters.push(filter);
    filters.push(filter2);

    let system = PFSystem {
        filters,
        sources: vec![source, second_source],
        sinks: vec![sink],
    };

    for sample_size in [100_000, 1_000_000, 10_000_000] {
        info!("Working on sample size {}", sample_size);
        for x in 0..sample_size {
            system.push(x % 2, x as f32);
        }

        let mut results = 0;
        let start = std::time::Instant::now();
        loop {
            system.run();
            if system.get_sink(0).borrow_mut().pop().is_some() {
                results += 1;
            }

            if results == 50 {
                break;
            }
        }
        let elapsed = start.elapsed();
        info!("Took {}ms", elapsed.as_millis());

        assert!(elapsed.as_millis() < 1000, "Test went over time !");
    }
}
