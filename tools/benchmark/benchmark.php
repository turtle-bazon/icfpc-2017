#!/usr/bin/env php
<?php

declare(ticks = 1);

require_once __DIR__ . '/vendor/autoload.php';
require_once __DIR__ . '/server.php';
require_once __DIR__ . '/array-to-texttable.php';

$specs = null;

$solverBinary = null;

function parseArgs() {
  global $specs,$argv,$solverBinary;

  $specs = new \GetOptionKit\OptionCollection();
  $specs
    ->add('m|maps?', 'Maps to run benchmark on. Comma-separated list of maps (without .json extension)' )
    ->isa('String')
    ->defaultValue('ALL');

  $specs
    ->add('r|rounds?', 'How many rounds to play on every map to run the benchmark.')
    ->isa('Number')
    ->defaultValue(10);

  $specs// tick use required as of PHP 4.3.0
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
    return $result;
  }
  catch(Exception $e) {
    echo $e->getMessage() . "\n";
    usage();
    exit(1);
  }
}

function usage() {
  global $specs,$argv;

  echo "Usage:\n\t{$argv[0]} [options] <solver-binary>\n\nOptions:\n";

  $printer = new \GetOptionKit\OptionPrinter\ConsoleOptionPrinter();
  echo $printer->render($specs);
}

function reportInit($maps, $rounds) {
  return [
    'maps' => array_fill_keys($maps, [
      'rounds' => 0,
      'roundsLeft' => $rounds,
      'games' => [],
      'summary' => [
        'errors' => 0,
        'avgScore' => 0,
        'avgMetaScore' => 0,
        'metaScore' => 0,
        'winRate' => 0,
      ],
    ]),
  ];
}

function reportPrintHuman($report) {
  $data = [];
  foreach($report['maps'] as $name => $info) {

    $data[] = [
      'Map' => $name,
      'Games' => count($info['games']),
      'Errors' => $info['summary']['errors'],
      'Score' => $info['summary']['avgScore'],
      'MetaScore' => $info['summary']['avgMetaScore'],
      'TotalMetaScore' => $info['summary']['metaScore'],
      'WinRate' => $info['summary']['winRate'],
    ];
  }

  $renderer = new ArrayToTextTable($data);
  $renderer->showHeaders(true);
  $renderer->render();
  echo "\n";
}

function reportReady(&$report) {
  foreach($report['maps'] as &$mapInfo) {
    if ($mapInfo['roundsLeft'] > 0)
      return false;
  }

  return true;
}

function reportUpdateGame(&$report, $game) {
  echo "[" . $game['pid'] . "] Game finished: " . $game['status'];

  $map = $game['map'];
  if ($game['status'] == 'error') {
    $report['maps'][$map]['roundsLeft']++;
    $report['maps'][$map]['summary']['errors']++;
    $report['maps'][$map]['games'][] = $game;
    echo "\n";
  }
  else {
    /* Parse score here... */
    $output = explode("\n", trim(file_get_contents($game['output'])));

    $myPunterID = -1; $score = null;
    foreach($output as $line) {
      if (($myPunterID == -1) && (($pos = strpos($line, '{"ready":')) !== false)) {
         $str = substr($line, $pos);
         $data = json_decode($str, true);
         $myPunterID = $data['ready'];
         continue;
      }
      if (!$score && (($pos = strpos($line, '{"stop":')) !== false)) {
         $str = substr($line, $pos);
         $data = json_decode($str, true);
         $score = $data['stop']['scores'];
         continue;
      }
    }

    $game['playerIndex'] = $myPunterID;
    $game['players'] = count($score);

    usort($score, function ($l, $r) {
      return $r['score'] - $l['score'];
    });

    $game['maxScore'] = $score[0]['score'];
    $metaScore = $game['players'];
    foreach($score as $row) {
      if ($row['score'] < $game['maxScore']) {
        $metaScore--;
      }

      if ($row['punter'] == $myPunterID) {
        $game['score'] = $row['score'];
        $game['metaScore'] = $metaScore;
        break;
      }
    }

    $game['result'] = ($metaScore == $game['players']) ? 'win' : 'loose';

    echo ": Score: " . $game['score'] . ", meta-score: " . $game['metaScore'] . " -> " . $game['result'] . "\n";
    $report['maps'][$map]['games'][] = $game;
  }

  unlink($game['output']);
}

function reportFinalize(&$report) {
  foreach($report['maps'] as &$mapInfo) {
    $score = 0; $metaScore = 0; $wins = 0; $gameCount = 0;

    foreach($mapInfo['games'] as $game){
      if ($game['status'] != 'ok') {
        continue;
      }
      $gameCount++;

      $score += $game['score'];
      $metaScore += $game['metaScore'];
      if ($game['result'] == 'win') {
        $wins++;
      }
    }


    if ($gameCount > 0) {
      $mapInfo['summary']['avgScore'] = ((float) $score) / $gameCount;
      $mapInfo['summary']['avgMetaScore'] = ((float) $metaScore) / $gameCount;
      $mapInfo['summary']['metaScore'] = $metaScore;
      $mapInfo['summary']['winRate'] = ((float) $wins) / $gameCount;
    }
  }
}

function allocateNextGame(&$report, &$srvList) {
  foreach($report['maps'] as $mapName => &$mapInfo) {
    if ($mapInfo['roundsLeft'] <= 0) {
      continue;
    }

    $srvSlot = srvAllocateSlot($srvList, $mapName);

    /* No free server for the map */
    if ($srvSlot === false) {
      continue;
    }

    $game = [
      'server' => $srvSlot,
      'map' => $mapName,

      'players' => -1,
      'playerIndex' => -1,
      'maxScore' => 0,
      'score' => 0,
      'metaScore' => 0,
      'gameResult' => null,
    ];

    $mapInfo['roundsLeft']--;
    return $game;
  }

  return false;
}

function runGame($game) {
  $output = tempnam('/tmp', 'game');
  $game['output'] = $output;

  $solver = $game['binary'];
  $args = [$game['server']['port'], $output];

  /* Time to fork... */
  switch ($pid = pcntl_fork()) {
    case -1:
      throw new Exception('Fork failed');
      break;

    case 0:
      pcntl_exec($solver, $args);
      break;

    default:
      echo "[{$pid}] Run a game on port " . $game['server']['port'] . ", map: " . $game['map'] . " => {$output}...\n";
      $game['pid'] = $pid;
      $game['status'] = 'running';
      break;
  }


  return $game;
}

$keepGoing = true;

pcntl_signal(SIGINT, function () use(&$keepGoing) {
  echo "Got INTERRUPT. Finalizing...\n";
  $keepGoing = false;
});

$result = parseArgs();

$status = srvFetchStatus();
$srvList = $status['servers'];

$sMaps = array_filter(array_map('trim', explode(',', $result->maps)), 'strlen');

if(in_array('ALL', $sMaps)) {
  $sMaps = array_map(function (&$s) {
    return $s['map']['shortName'];
  }, $srvList);
}

$sMaps = array_unique($sMaps);
$report = reportInit($sMaps, $result->rounds);

$games = [];

$status = srvFetchStatus();
$srvList = srvPrepareList($status['servers']);
while ($keepGoing && !reportReady($report)) {
  while (count($games) < $result->parallel) {
    $game = allocateNextGame($report, $srvList);
    if ($game === false) {
      break;
    }

    $game['binary'] = $solverBinary;
    $game = runGame($game);
    $games[$game['pid']] = $game;
  }

  /* OK. All parallel slots are busy. */
  while (count($games) >= $result->parallel) {
    $status = 0;
    while (($chPID = pcntl_waitpid(-1, $status, WNOHANG)) > 0) {
      if (!isset($games[$chPID])) {
        echo "Strange unknown child pid exited {$chPID} with status {$status}. Ignoring...\n";
        continue;
      }

      $game = $games[$chPID];
      $game['status'] = ($status == 0) ? 'ok' : 'error';
      reportUpdateGame($report, $game);
      unset($games[$chPID]);
    }

    if ($keepGoing) {
      sleep(5);
      $status = srvFetchStatus();
      $srvList = srvPrepareList($status['servers']);
    }
  }
}

reportFinalize($report);
reportPrintHuman($report);
