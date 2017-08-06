#!/usr/bin/env php
<?php

require_once __DIR__ . '/parse-status.php';


function usage() {
  global $specs,$argv;

  echo "Usage:\n\t{$argv[0]} [options] <solver-binary>\n\nOptions:\n";

  $printer = new \GetOptionKit\OptionPrinter\ConsoleOptionPrinter();
  echo $printer->render($specs);
}

$specs = new \GetOptionKit\OptionCollection();
$specs
  ->add('m|maps?', 'Maps to run benchmark on. Comma-separated list of maps (without .json extension)' )
  ->isa('String')
  ->defaultValue('ALL');

$specs
  ->add('r|rounds?', 'How many rounds to play on every map to run the benchmark.')
  ->isa('Number')
  ->defaultValue(10);

$specs
  ->add('p|parallel?', 'Number of concurrent running games.')
  ->isa('Number')
  ->defaultValue(2);

$specs
  ->add('help', 'Show usage help');

try {
  $parser = new \GetOptionKit\OptionParser($specs);
  $result = $parser->parse($argv);
  if ($result->help) {
    usage(); exit(0);
  }

  $args = $result->getArguments();
  if(count($args) != 1) {
    throw new Exception('Bad solver binary path');
  }
  $solverBinary = $args[0];
}
catch(Exception $e) {
  echo $e->getMessage() . "\n";
  usage();
  exit(1);
}
