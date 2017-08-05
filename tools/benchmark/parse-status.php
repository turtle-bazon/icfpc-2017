#!/usr/bin/env php
<?php

require_once __DIR__ . '/vendor/autoload.php';

$curl = new \Curl\Curl();
$curl->setUserAgent('SkobochkaBenchmarkBot/1.0.0 (+http://icfpc.gnoll.tech/bot.html)');
$curl->setOpt(CURLOPT_FOLLOWLOCATION, true);

$curl->get('http://punter.inf.ed.ac.uk/status.html');
if ($curl->error) {
  echo json_encode(['result' => false, 'code' => $curl->errorCode, 'message' => $curl->errorMessage]);
  exit(1);
}

$doc = DOMDocument::loadHTML($curl->response);

if ($doc === false) {
  echo json_encode(['result' => false, 'code' => -1, 'message' => 'Unable to parse input']);
  exit(1);
}

$xpath = new DOMXpath($doc);

$match = [];

$elements = $xpath->query("//h3");
$genInfoRaw = $elements->item(0)->nodeValue;
if (preg_match('/Information generated: (.+)$/ui', $genInfoRaw, $matched)) {
  $ts = $matched[1] . '+02:00';
  $ts = new DateTime($ts);
  $ts->setTimezone(new DateTimeZone('UTC'));
}


$elements = $xpath->query("//tr");

$data = [];

if (!is_null($elements)) {
  foreach ($elements as $element) {

    $nodes = $element->childNodes;

    $nowPunters = -1; $maxPunters = -1;
    $srvStatusRaw = $nodes->item(0)->nodeValue;
    $srvStatus = 'UNKNOWN';
    if ($srvStatusRaw == 'Offline.') {
      $srvStatus = 'OFFLINE';
    }
    elseif ($srvStatusRaw == 'Game in progress.') {
      $srvStatus = 'IN_PROGRESS';
    }
    elseif (preg_match('/Waiting for punters\. \(([0-9]+)\/([0-9]+)\)/ui', $srvStatusRaw, $match) > 0) {
      $srvStatus = 'WAIT';
      $nowPunters = (int)($match[1]); $maxPunters = (int)($match[2]);
    }
    $puntersRaw = $nodes->item(1)->nodeValue;
    $punters = array_filter(array_map('trim', explode(',', $puntersRaw)), 'strlen');

    $extensionsRaw = $nodes->item(2)->nodeValue;
    $extensions = array_filter(array_map('trim', explode(',', $extensionsRaw)), 'strlen');

    $data[] = [
      'status' => $srvStatus,
      'punters' => [
        'names' => $punters,
        'count' => $nowPunters,
        'maxCount' => $maxPunters,
      ],
      'extensions' => $extensions,
      'port' => (int)($nodes->item(3)->nodeValue),
      'map' => [
        'name' => $nodes->item(4)->nodeValue,
        'url' => 'http://punter.inf.ed.ac.uk/maps/' . $nodes->item(4)->nodeValue,
      ],
    ];
  }
}

echo json_encode(['result' => true, 'ts' => $ts->format(DateTime::ISO8601) ,'servers' => $data]);
